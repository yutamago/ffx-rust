//! A cluster taken apart for structural edits: objects added to instance
//! lists, arrays grown, fixups added or retargeted. [`ClusterEdit::write`]
//! puts it back together.
//!
//! What is kept as bytes: the header and namespace, each list's object and
//! array data, the user fixups and header class tables, and the buffers
//! that end the file. What is decoded: the three kinds of fixup, per list.
//! Writing recomputes every count and size the header and list headers
//! hold, and packs the fixups as explicit records (group mode 5). The
//! shipped files use compact modes where they fit, so a written cluster is
//! larger than the original by a few bytes per fixup even when nothing
//! changed, but unpacks to the same records.
//!
//! Objects are addressed by (list, index). Inserting objects shifts the
//! indices after them in every fixup that refers to that list, so pointers
//! keep their targets. An array pointer (count > 1) spanning the insertion
//! point is left to the caller, who knows whether the new object belongs
//! to the array.

use crate::cluster::Cluster;
use crate::fixup::{
    ArrayFixup, PointerFixup, pack_array_fixups, pack_pointer_fixups, unpack_array_fixups,
};
use crate::{Error, HEADER_LEN, Result};

const LIST_HEADER_LEN: usize = 0x24;

/// One instance list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditList {
    /// The list header as stored; counts and sizes are rewritten.
    header: [u8; LIST_HEADER_LEN],
    /// Index of the list's class in [`Cluster::classes`].
    pub class: usize,
    pub object_size: usize,
    /// The objects, back to back.
    pub objects: Vec<u8>,
    /// The array data that follows the objects.
    pub arrays: Vec<u8>,
    pub pointer_fixups: Vec<PointerFixup>,
    pub array_fixups: Vec<ArrayFixup>,
    pub pointer_array_fixups: Vec<ArrayFixup>,
    /// Whether the list's data was a multiple of 4 bytes; writing keeps
    /// it so (shader clusters have lists that are not).
    aligned: bool,
}

impl EditList {
    /// Number of objects.
    #[must_use]
    pub fn count(&self) -> u32 {
        u32::try_from(
            self.objects
                .len()
                .checked_div(self.object_size)
                .unwrap_or(0),
        )
        .expect("object count")
    }

    /// The bytes of object `i`.
    #[must_use]
    pub fn object(&self, i: u32) -> &[u8] {
        let at = i as usize * self.object_size;
        &self.objects[at..at + self.object_size]
    }

    /// The bytes of object `i`, writable.
    pub fn object_mut(&mut self, i: u32) -> &mut [u8] {
        let at = i as usize * self.object_size;
        &mut self.objects[at..at + self.object_size]
    }
}

/// A cluster taken apart.
#[derive(Debug, Clone)]
pub struct ClusterEdit {
    /// The parsed tables (namespace, classes, user fixups) of the original.
    pub cluster: Cluster,
    /// Header and namespace bytes; header words are rewritten.
    head: Vec<u8>,
    pub lists: Vec<EditList>,
    /// The user fixup data (strings and two-byte asset indices).
    pub user_data: Vec<u8>,
    /// User fixups: (type, length, offset into [`ClusterEdit::user_data`]).
    pub user_records: Vec<[u32; 3]>,
    /// One count per `PParameterBuffer` list, in list order: its entries
    /// in [`ClusterEdit::header_class_fixups`].
    pub header_class_counts: Vec<u32>,
    /// `(0, offset, 0, size)` per parameter of those buffers.
    pub header_class_fixups: Vec<[u32; 4]>,
    /// Everything after the packed fixups (texture pixels; index then
    /// vertex buffer).
    pub buffers: Vec<u8>,
}

fn u32_at(b: &[u8], o: usize) -> Result<u32> {
    b.get(o..o + 4)
        .map(|s| u32::from_le_bytes(s.try_into().expect("4 bytes")))
        .ok_or(Error::Malformed("truncated cluster"))
}

fn put(b: &mut [u8], o: usize, v: usize) -> Result<()> {
    let v = u32::try_from(v).map_err(|_| Error::Unsupported("cluster over 4 GiB".into()))?;
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
    Ok(())
}

impl ClusterEdit {
    /// Takes a cluster apart.
    ///
    /// # Errors
    /// Fails on clusters [`Cluster::parse`] or the fixup unpackers reject.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let cluster = Cluster::parse(bytes)?;
        let h = &cluster.header;
        let lists_at = HEADER_LEN + h.namespace_size as usize;
        let pointer_fixups = cluster.pointer_fixups(bytes)?;
        let array_fixups = cluster.array_fixups(bytes)?;
        // Pointer-array fixups come first in the packed block; their
        // per-list counts are at list header +0x20.
        let pa_counts: Vec<(u32, u32)> = (0..cluster.lists.len())
            .map(|i| {
                Ok((
                    u32_at(bytes, lists_at + LIST_HEADER_LEN * i + 0x20)?,
                    cluster.lists[i].count,
                ))
            })
            .collect::<Result<_>>()?;
        let pa_size = h.pointer_array_fixup_size as usize;
        let packed = bytes
            .get(cluster.fixups_offset..cluster.fixups_offset + pa_size)
            .ok_or(Error::Malformed("pointer-array fixups outside the file"))?;
        let (flat, used) = unpack_array_fixups(packed, &pa_counts)?;
        if used != pa_size {
            return Err(Error::Malformed("pointer-array fixups do not add up"));
        }
        let mut flat = flat.into_iter();

        let mut lists = Vec::with_capacity(cluster.lists.len());
        for (i, (l, (pf, af))) in cluster
            .lists
            .iter()
            .zip(pointer_fixups.into_iter().zip(array_fixups))
            .enumerate()
        {
            let at = lists_at + LIST_HEADER_LEN * i;
            let header: [u8; LIST_HEADER_LEN] = bytes[at..at + LIST_HEADER_LEN]
                .try_into()
                .expect("list header");
            let objects = bytes[l.offset..l.offset + l.objects_size as usize].to_vec();
            let arrays = bytes[l.arrays()].to_vec();
            lists.push(EditList {
                header,
                class: l.class,
                object_size: l.object_size(),
                objects,
                arrays,
                pointer_fixups: pf,
                array_fixups: af,
                pointer_array_fixups: flat.by_ref().take(pa_counts[i].0 as usize).collect(),
                aligned: (l.objects_size + l.arrays_size).is_multiple_of(4),
            });
        }
        let data_end =
            lists_at + LIST_HEADER_LEN * cluster.lists.len() + h.total_data_size as usize;
        let buffers_at = cluster.fixups_offset
            + pa_size
            + h.pointer_fixup_size as usize
            + h.array_fixup_size as usize;
        Ok(Self {
            head: bytes[..lists_at].to_vec(),
            lists,
            user_data: bytes[data_end..data_end + h.user_fixup_data_size as usize].to_vec(),
            user_records: (0..h.user_fixup_count as usize)
                .map(|i| {
                    let at = data_end + h.user_fixup_data_size as usize + 12 * i;
                    Ok([
                        u32_at(bytes, at)?,
                        u32_at(bytes, at + 4)?,
                        u32_at(bytes, at + 8)?,
                    ])
                })
                .collect::<Result<_>>()?,
            header_class_counts: {
                let at =
                    data_end + h.user_fixup_data_size as usize + 12 * h.user_fixup_count as usize;
                (0..h.header_class_count as usize)
                    .map(|i| u32_at(bytes, at + 4 * i))
                    .collect::<Result<_>>()?
            },
            header_class_fixups: {
                let at = data_end
                    + h.user_fixup_data_size as usize
                    + 12 * h.user_fixup_count as usize
                    + 4 * h.header_class_count as usize;
                (0..h.header_class_fixup_count as usize)
                    .map(|i| {
                        let o = at + 16 * i;
                        Ok([
                            u32_at(bytes, o)?,
                            u32_at(bytes, o + 4)?,
                            u32_at(bytes, o + 8)?,
                            u32_at(bytes, o + 12)?,
                        ])
                    })
                    .collect::<Result<_>>()?
            },
            buffers: bytes
                .get(buffers_at..)
                .ok_or(Error::Malformed("buffers outside the file"))?
                .to_vec(),
            cluster,
        })
    }

    /// Index of the first list of class `name`.
    #[must_use]
    pub fn list_of(&self, name: &str) -> Option<usize> {
        self.lists
            .iter()
            .position(|l| self.cluster.classes[l.class].name == name)
    }

    /// Inserts `count` copies of `template` (one object's bytes) into list
    /// `list` before object `at`, and shifts every fixup index that refers
    /// to objects from `at` on. Returns the index of the first new object.
    ///
    /// # Errors
    /// Fails if the template has the wrong size or `at` is past the end.
    pub fn insert_objects(
        &mut self,
        list: usize,
        at: u32,
        template: &[u8],
        count: u32,
    ) -> Result<u32> {
        let l = &mut self.lists[list];
        if template.len() != l.object_size || at > l.count() {
            return Err(Error::Unsupported("object does not fit the list".into()));
        }
        let pos = at as usize * l.object_size;
        let new: Vec<u8> = template.repeat(count as usize);
        l.objects.splice(pos..pos, new);
        let shift = |i: &mut u32| {
            if *i >= at {
                *i += count;
            }
        };
        for (k, l) in self.lists.iter_mut().enumerate() {
            if k == list {
                for f in &mut l.pointer_fixups {
                    shift(&mut f.0[1]);
                }
                for f in l.array_fixups.iter_mut().chain(&mut l.pointer_array_fixups) {
                    shift(&mut f.0[1]);
                }
            }
            for f in &mut l.pointer_fixups {
                if f.user_fixup().is_none() && f.0[3] as usize == list {
                    shift(&mut f.0[2]);
                }
            }
        }
        Ok(at)
    }

    /// Appends `data` to list `list`'s array data, 4-byte aligned, and
    /// returns its offset there (the value of an array fixup's word 3).
    pub fn append_array(&mut self, list: usize, data: &[u8]) -> u32 {
        let arrays = &mut self.lists[list].arrays;
        arrays.resize(arrays.len().next_multiple_of(4), 0);
        let at = arrays.len();
        arrays.extend_from_slice(data);
        u32::try_from(at).expect("array data under 4 GiB")
    }

    /// Puts the cluster back together.
    ///
    /// # Errors
    /// Fails if a fixup cannot be encoded or the file outgrows 32 bits.
    pub fn write(&self) -> Result<Vec<u8>> {
        let mut out = self.head.clone();
        let mut data = Vec::new();
        for l in &self.lists {
            // Model lists' data is a multiple of 4 bytes (the loader lays
            // the lists out back to back; a string of odd length misaligned
            // every list after it and crashed the game).
            let mut arrays = l.arrays.clone();
            if l.aligned {
                arrays.resize(
                    (l.objects.len() + arrays.len()).next_multiple_of(4) - l.objects.len(),
                    0,
                );
            }
            let mut header = l.header;
            put(&mut header, 4, l.count() as usize)?;
            put(&mut header, 8, l.objects.len() + arrays.len())?;
            put(&mut header, 0xc, l.objects.len())?;
            put(&mut header, 0x10, arrays.len())?;
            // The elements of the list's pointer arrays.
            put(
                &mut header,
                0x14,
                l.pointer_array_fixups.iter().map(|f| f.0[2] as usize).sum(),
            )?;
            put(&mut header, 0x18, l.array_fixups.len())?;
            put(&mut header, 0x1c, l.pointer_fixups.len())?;
            put(&mut header, 0x20, l.pointer_array_fixups.len())?;
            out.extend_from_slice(&header);
            data.extend_from_slice(&l.objects);
            data.extend_from_slice(&arrays);
        }
        let total_data = data.len();
        out.extend(data);
        out.extend_from_slice(&self.user_data);
        for r in self
            .user_records
            .iter()
            .map(|r| &r[..])
            .chain(std::iter::once(&self.header_class_counts[..]))
        {
            for v in r {
                out.extend(v.to_le_bytes());
            }
        }
        for f in &self.header_class_fixups {
            for v in f {
                out.extend(v.to_le_bytes());
            }
        }

        let counts: Vec<u32> = self.lists.iter().map(EditList::count).collect();
        let pa: Vec<(&[ArrayFixup], u32)> = self
            .lists
            .iter()
            .zip(&counts)
            .map(|(l, &n)| (l.pointer_array_fixups.as_slice(), n))
            .collect();
        let pp: Vec<(&[PointerFixup], u32)> = self
            .lists
            .iter()
            .zip(&counts)
            .map(|(l, &n)| (l.pointer_fixups.as_slice(), n))
            .collect();
        let aa: Vec<(&[ArrayFixup], u32)> = self
            .lists
            .iter()
            .zip(&counts)
            .map(|(l, &n)| (l.array_fixups.as_slice(), n))
            .collect();
        let (pa, pp, aa) = (
            pack_array_fixups(&pa)?,
            pack_pointer_fixups(&pp)?,
            pack_array_fixups(&aa)?,
        );
        let sum = |f: &dyn Fn(&EditList) -> usize| self.lists.iter().map(f).sum::<usize>();
        let words = [
            (4, self.lists.len()),
            (5, aa.len()),
            (6, sum(&|l| l.array_fixups.len())),
            (7, pp.len()),
            (8, sum(&|l| l.pointer_fixups.len())),
            (9, pa.len()),
            (10, sum(&|l| l.pointer_array_fixups.len())),
            // Word 11: the elements of all pointer arrays.
            (
                11,
                sum(&|l| l.pointer_array_fixups.iter().map(|f| f.0[2] as usize).sum()),
            ),
            (12, self.user_records.len()),
            (13, self.user_data.len()),
            (14, total_data),
            (15, self.header_class_counts.len()),
            (16, self.header_class_fixups.len()),
        ];
        for (w, v) in words {
            put(&mut out, 4 * w, v)?;
        }
        out.extend(pa);
        out.extend(pp);
        out.extend(aa);
        out.extend_from_slice(&self.buffers);
        Ok(out)
    }
}

/// Where a pointer lives in its object: bit 31 marks a byte offset,
/// otherwise the word is a member index ([`Cluster::member_by_index`]).
fn field_offset(cluster: &Cluster, word: u32) -> Option<u32> {
    if word & 0x8000_0000 != 0 {
        Some(word & 0x7fff_ffff)
    } else {
        cluster.member_by_index(word).map(|(_, m)| m.offset)
    }
}

impl ClusterEdit {
    /// The pointer fixup of object `obj` in `list` at byte `offset`.
    fn pointer_at(&self, list: usize, obj: u32, offset: u32) -> Option<usize> {
        self.lists[list].pointer_fixups.iter().position(|f| {
            f.object() == obj && field_offset(&self.cluster, f.member()) == Some(offset)
        })
    }

    /// Byte offset of `member` in objects of the class of `list`.
    fn offset_of(&self, list: usize, member: &str) -> Result<u32> {
        let class = &self.cluster.classes[self.lists[list].class].name;
        self.cluster
            .member_offset(class, member)
            .ok_or_else(|| Error::Unsupported(format!("{class} has no {member}")))
    }

    fn list_named(&self, name: &str) -> Result<usize> {
        self.list_of(name)
            .ok_or_else(|| Error::Unsupported(format!("no {name} list")))
    }

    fn add_u32(&mut self, list: usize, obj: u32, offset: u32, delta: u32) {
        let o = offset as usize;
        let b = &mut self.lists[list].object_mut(obj)[o..o + 4];
        let v = u32::from_le_bytes(b[..4].try_into().expect("4 bytes")) + delta;
        b.copy_from_slice(&v.to_le_bytes());
    }

    /// Appends copies of objects `first..first + count` of `list` with the
    /// fixups they hold (pointers keep their targets); returns the index of
    /// the first copy.
    fn copy_objects(&mut self, list: usize, first: u32, count: u32) -> u32 {
        let l = &mut self.lists[list];
        let at = l.count();
        let size = l.object_size;
        let bytes = l.objects[first as usize * size..(first + count) as usize * size].to_vec();
        l.objects.extend(bytes);
        let moved = |o: u32| (o >= first && o < first + count).then(|| o - first + at);
        let p: Vec<PointerFixup> = l
            .pointer_fixups
            .iter()
            .filter_map(|f| {
                moved(f.object()).map(|o| {
                    let mut g = *f;
                    g.0[1] = o;
                    g
                })
            })
            .collect();
        l.pointer_fixups.extend(p);
        for fixups in [&mut l.array_fixups, &mut l.pointer_array_fixups] {
            let a: Vec<ArrayFixup> = fixups
                .iter()
                .filter_map(|f| {
                    moved(f.object()).map(|o| {
                        let mut g = *f;
                        g.0[1] = o;
                        g
                    })
                })
                .collect();
            fixups.extend(a);
        }
        at
    }

    /// Gives the array pointer at `offset` of object `obj` its own copy of
    /// the objects it points to, appended to their list; returns (list,
    /// first copy, count).
    fn own_copy(&mut self, list: usize, obj: u32, offset: u32) -> Result<(usize, u32, u32)> {
        let k = self
            .pointer_at(list, obj, offset)
            .ok_or(Error::Unsupported("array pointer without a fixup".into()))?;
        let f = self.lists[list].pointer_fixups[k];
        let (target, first, count) = (f.target_list() as usize, f.target(), f.count().max(1));
        let copy = self.copy_objects(target, first, count);
        self.lists[list].pointer_fixups[k].0[2] = copy;
        Ok((target, copy, count))
    }

    /// Adds a mesh segment: a copy of segment `template` (its object, bone
    /// palette, vertex data blocks and their streams), placed right after
    /// the last segment of the same mesh, with a segment context added to
    /// every instance of the mesh. Its geometry starts as the template's;
    /// set it through [`crate::Model`] and [`crate::Model::write`]. Returns
    /// the new segment's index in the `PMeshSegment` list.
    ///
    /// # Errors
    /// Fails on clusters without the model classes, or a template no mesh
    /// lists.
    pub fn add_segment(&mut self, template: u32) -> Result<u32> {
        let (segs, meshes, insts) = (
            self.list_named("PMeshSegment")?,
            self.list_named("PMesh")?,
            self.list_named("PMeshInstance")?,
        );
        let mesh_array = self.offset_of(meshes, "m_meshSegments")?;
        // The mesh whose segment range holds the template.
        let (mesh, k) = (0..self.lists[meshes].count())
            .find_map(|m| {
                let k = self.pointer_at(meshes, m, mesh_array + 4)?;
                let f = self.lists[meshes].pointer_fixups[k];
                (f.target_list() as usize == segs
                    && (f.target()..f.target() + f.count()).contains(&template))
                .then_some((m, k))
            })
            .ok_or(Error::Unsupported(
                "no mesh lists the template segment".into(),
            ))?;
        let f = self.lists[meshes].pointer_fixups[k];
        let new = f.target() + f.count();
        let bytes = self.lists[segs].object(template).to_vec();
        self.insert_objects(segs, new, &bytes, 1)?;
        let template = if template >= new {
            template + 1
        } else {
            template
        };
        self.lists[meshes].pointer_fixups[k].0[5] += 1;
        self.add_u32(meshes, mesh, mesh_array, 1);
        // The template's pointers, then copies of what they point to.
        let held: Vec<PointerFixup> = self.lists[segs]
            .pointer_fixups
            .iter()
            .filter(|f| f.object() == template)
            .map(|f| {
                let mut g = *f;
                g.0[1] = new;
                g
            })
            .collect();
        self.lists[segs].pointer_fixups.extend(held);
        let bones = self.offset_of(segs, "m_skinBones")?;
        if self.pointer_at(segs, new, bones + 4).is_some() {
            self.own_copy(segs, new, bones + 4)?;
        }
        let vertex_data = self.offset_of(segs, "m_vertexData")?;
        let (blocks, first, count) = self.own_copy(segs, new, vertex_data + 4)?;
        let streams = self.offset_of(blocks, "m_streams")?;
        for b in first..first + count {
            if self.pointer_at(blocks, b, streams + 4).is_some() {
                self.own_copy(blocks, b, streams + 4)?;
            }
        }
        // A segment context for every instance of the mesh.
        let (inst_mesh, contexts) = (
            self.offset_of(insts, "m_mesh")?,
            self.offset_of(insts, "m_segmentContext")?,
        );
        for i in 0..self.lists[insts].count() {
            let Some(m) = self.pointer_at(insts, i, inst_mesh) else {
                continue;
            };
            let m = self.lists[insts].pointer_fixups[m];
            if m.target_list() as usize != meshes || m.target() != mesh {
                continue;
            }
            let Some(c) = self.pointer_at(insts, i, contexts + 4) else {
                continue;
            };
            let cf = self.lists[insts].pointer_fixups[c];
            let list = cf.target_list() as usize;
            let at = cf.target() + cf.count();
            let bytes = self.lists[list].object(cf.target()).to_vec();
            self.insert_objects(list, at, &bytes, 1)?;
            self.lists[insts].pointer_fixups[c].0[5] += 1;
            self.add_u32(insts, i, contexts, 1);
        }
        Ok(new)
    }
}

impl ClusterEdit {
    /// The mesh whose segment range holds `segment`: (mesh index, its
    /// `m_meshSegments` fixup).
    pub(crate) fn mesh_of(&self, segment: u32) -> Result<(u32, usize)> {
        let (segs, meshes) = (self.list_named("PMeshSegment")?, self.list_named("PMesh")?);
        let array = self.offset_of(meshes, "m_meshSegments")?;
        (0..self.lists[meshes].count())
            .find_map(|m| {
                let k = self.pointer_at(meshes, m, array + 4)?;
                let f = self.lists[meshes].pointer_fixups[k];
                (f.target_list() as usize == segs
                    && (f.target()..f.target() + f.count()).contains(&segment))
                .then_some((m, k))
            })
            .ok_or(Error::Unsupported("no mesh lists the segment".into()))
    }

    /// Gives `segment` the bone palette `bones` (hierarchy matrix, skeleton
    /// matrix): new `PSkinBoneRemap`s at the end of their list; the old
    /// entries stay unused.
    ///
    /// # Errors
    /// Fails on clusters without skinned segments.
    pub fn set_palette(&mut self, segment: u32, bones: &[(u16, u16)]) -> Result<()> {
        let segs = self.list_named("PMeshSegment")?;
        let remaps = self.list_named("PSkinBoneRemap")?;
        let field = self.offset_of(segs, "m_skinBones")?;
        let k = self
            .pointer_at(segs, segment, field + 4)
            .ok_or(Error::Unsupported("segment without a bone palette".into()))?;
        let (hier, skel) = (
            self.offset_of(remaps, "m_hierarchyMatrixIndex")? as usize,
            self.offset_of(remaps, "m_skeletonMatrixIndex")? as usize,
        );
        let l = &mut self.lists[remaps];
        let first = l.count();
        let mut template = l.object(0).to_vec();
        for &(h, s) in bones {
            template[hier..hier + 2].copy_from_slice(&h.to_le_bytes());
            template[skel..skel + 2].copy_from_slice(&s.to_le_bytes());
            l.objects.extend_from_slice(&template);
        }
        let count = u32::try_from(bones.len()).map_err(|_| Error::Unsupported("palette".into()))?;
        let f = &mut self.lists[segs].pointer_fixups[k];
        f.0[2] = first;
        f.0[5] = count;
        let o = field as usize;
        self.lists[segs].object_mut(segment)[o..o + 4].copy_from_slice(&count.to_le_bytes());
        Ok(())
    }

    /// Adds a skeleton matrix for hierarchy joint `hierarchy` (its inverse
    /// bind matrix, column-major) and a joint bounds entry (a copy of the
    /// last, naming `hierarchy`) to the mesh holding `segment`; returns its
    /// skeleton matrix index.
    ///
    /// # Errors
    /// Fails on meshes without a skeleton.
    pub fn add_skeleton_matrix(
        &mut self,
        segment: u32,
        hierarchy: u16,
        matrix: &[f32; 16],
    ) -> Result<u16> {
        let (mesh, _) = self.mesh_of(segment)?;
        let meshes = self.list_named("PMesh")?;
        let mut index = None;
        for member in ["m_skeletonMatrices", "m_skeletonBounds"] {
            let field = self.offset_of(meshes, member)?;
            let k = self
                .pointer_at(meshes, mesh, field + 4)
                .ok_or(Error::Unsupported("mesh without a skeleton".into()))?;
            let f = self.lists[meshes].pointer_fixups[k];
            let list = f.target_list() as usize;
            let at = f.target() + f.count();
            let mut bytes = self.lists[list].object(at - 1).to_vec();
            if member == "m_skeletonMatrices" {
                for (i, v) in matrix.iter().enumerate() {
                    bytes[4 * i..4 * i + 4].copy_from_slice(&v.to_le_bytes());
                }
                index = Some(f.count());
            } else if let Some(o) = self
                .cluster
                .member_offset("PSkeletonJointBounds", "m_hierarchyMatrixIndex")
            {
                let o = o as usize;
                bytes[o..o + 4].copy_from_slice(&u32::from(hierarchy).to_le_bytes());
            }
            self.insert_objects(list, at, &bytes, 1)?;
            self.lists[meshes].pointer_fixups[k].0[5] += 1;
            self.add_u32(meshes, mesh, field, 1);
        }
        u16::try_from(index.expect("set above")).map_err(|_| Error::Unsupported("skeleton".into()))
    }
}

impl ClusterEdit {
    /// Inserts a list before list `at`; fixups pointing into lists from
    /// `at` on follow them.
    fn insert_list(&mut self, at: usize, list: EditList) {
        for l in &mut self.lists {
            for f in &mut l.pointer_fixups {
                if f.user_fixup().is_none() && f.0[3] as usize >= at {
                    f.0[3] += 1;
                }
            }
        }
        let mut list = list;
        for f in &mut list.pointer_fixups {
            if f.user_fixup().is_none() && f.0[3] as usize >= at {
                f.0[3] += 1;
            }
        }
        self.lists.insert(at, list);
    }

    /// A NUL-terminated string in a list's array data.
    fn array_string(&self, list: usize, at: u32) -> String {
        let tail = self.lists[list]
            .arrays
            .get(at as usize..)
            .unwrap_or_default();
        let end = tail.iter().position(|&b| b == 0).unwrap_or(tail.len());
        String::from_utf8_lossy(&tail[..end]).into_owned()
    }

    /// The array fixup of object `obj` in `list` at byte `offset`.
    fn array_at(&self, list: usize, obj: u32, offset: u32) -> Option<usize> {
        self.lists[list].array_fixups.iter().position(|f| {
            f.object() == obj && field_offset(&self.cluster, f.member()) == Some(offset)
        })
    }

    /// Adds a material to the mesh holding `segment`: a copy of `PMaterial`
    /// `material` with its own copy of the parameter buffer (a new list,
    /// after the other buffers, with its header class entry), appended to
    /// the mesh's material set. Returns the new `PMaterial` index and its
    /// index in the set (the value for `PMeshSegment.m_materialIndex`).
    ///
    /// # Errors
    /// Fails on clusters without materials or a material without a buffer.
    pub fn add_material(&mut self, material: u32, segment: u32) -> Result<(u32, u32)> {
        let materials = self.list_named("PMaterial")?;
        let meshes = self.list_named("PMesh")?;
        let field = self.offset_of(materials, "m_parameterBuffer")?;
        let k = self
            .pointer_at(materials, material, field)
            .ok_or(Error::Unsupported(
                "material without a parameter buffer".into(),
            ))?;
        let buffer = self.lists[materials].pointer_fixups[k].target_list() as usize;
        // The new buffer list goes after the last buffer list, its header
        // class entry after the last buffer's (they come last, in list order).
        let buffers: Vec<usize> = (0..self.lists.len())
            .filter(|&i| self.cluster.classes[self.lists[i].class].name == "PParameterBuffer")
            .collect();
        let rank = buffers
            .iter()
            .position(|&b| b == buffer)
            .expect("a buffer list");
        let before = self
            .header_class_counts
            .len()
            .checked_sub(buffers.len())
            .ok_or(Error::Unsupported(
                "header classes do not match the buffers".into(),
            ))?;
        let first: usize = self.header_class_counts[..before + rank]
            .iter()
            .map(|&n| n as usize)
            .sum();
        let n = self.header_class_counts[before + rank];
        let entries = self.header_class_fixups[first..first + n as usize].to_vec();
        self.header_class_counts.push(n);
        self.header_class_fixups.extend(entries);
        let at = buffers.last().expect("a buffer list") + 1;
        let copy = self.lists[buffer].clone();
        self.insert_list(at, copy);
        let (materials, meshes) = (
            if materials >= at {
                materials + 1
            } else {
                materials
            },
            if meshes >= at { meshes + 1 } else { meshes },
        );
        let new = self.copy_objects(materials, material, 1);
        let k = self
            .pointer_at(materials, new, field)
            .expect("copied with the material");
        self.lists[materials].pointer_fixups[k].0[2] = 0;
        self.lists[materials].pointer_fixups[k].0[3] = u32::try_from(at).expect("list index");
        // The mesh's material set: a pointer array (count, then pointers).
        let (mesh, _) = self.mesh_of(segment)?;
        let set = self.offset_of(meshes, "m_defaultMaterials")?;
        let pa = self.lists[meshes]
            .pointer_array_fixups
            .iter()
            .position(|f| {
                f.object() == mesh && field_offset(&self.cluster, f.member()) == Some(set + 4)
            })
            .ok_or(Error::Unsupported("mesh without a material set".into()))?;
        let index = self.lists[meshes].pointer_array_fixups[pa].0[2];
        self.lists[meshes].pointer_array_fixups[pa].0[2] += 1;
        let template = *self.lists[meshes]
            .pointer_fixups
            .iter()
            .find(|f| {
                f.object() == mesh && field_offset(&self.cluster, f.member()) == Some(set + 4)
            })
            .ok_or(Error::Unsupported("empty material set".into()))?;
        let mut element = template;
        element.0[2] = new;
        element.0[3] = u32::try_from(materials).expect("list index");
        element.0[5] = index;
        self.lists[meshes].pointer_fixups.push(element);
        self.add_u32(meshes, mesh, set, 1);
        Ok((new, index))
    }

    /// Points sampler `sampler` (`TextureSampler` is the base color) of
    /// `PMaterial` `material` at texture `import` (an import id such as
    /// `PS3Data/chr/mon/m009/tex/m009.dds`, stored in the archive as
    /// `ffx_data/gamedata/ps3data/chr/mon/m009/tex/d3d11/m009.dds.phyre`),
    /// adding the import if the cluster has none for it.
    ///
    /// # Errors
    /// Fails if the material has no such sampler.
    pub fn set_texture(&mut self, material: u32, sampler: &str, import: &str) -> Result<()> {
        let (buffer, slot, old) = self.sampler(material, sampler)?;
        // The import: an existing one, else a copy of the current one.
        let index = match self.import_index(import)? {
            Some(i) => i,
            None => {
                let [_, _, off] = self.user_records[old as usize];
                let current = u16::from_be_bytes([
                    self.user_data[off as usize],
                    self.user_data[off as usize + 1],
                ]);
                self.add_import(u32::from(current), import)?
            }
        };
        self.point_sampler(buffer, slot, old, index)
    }

    /// The `PAssetReferenceImport` with id `import`, if there is one.
    ///
    /// # Errors
    /// Fails on clusters without imports.
    pub fn import_index(&self, import: &str) -> Result<Option<u32>> {
        let imports = self.list_named("PAssetReferenceImport")?;
        let id_field = self.offset_of(imports, "m_id")?;
        Ok((0..self.lists[imports].count()).find(|&i| {
            self.array_at(imports, i, id_field).is_some_and(|a| {
                self.array_string(imports, self.lists[imports].array_fixups[a].0[3]) == import
            })
        }))
    }

    /// Adds a `PAssetReferenceImport` like import `like` (same target type)
    /// with id `import`; returns its index.
    ///
    /// # Errors
    /// Fails on clusters without imports.
    pub fn add_import(&mut self, like: u32, import: &str) -> Result<u32> {
        let imports = self.list_named("PAssetReferenceImport")?;
        let id_field = self.offset_of(imports, "m_id")?;
        let i = self.copy_objects(imports, like, 1);
        let l = &mut self.lists[imports];
        let at = u32::try_from(l.arrays.len()).expect("array data");
        l.arrays.extend_from_slice(import.as_bytes());
        l.arrays.push(0);
        let a = self
            .array_at(imports, i, id_field)
            .ok_or(Error::Unsupported("import without an id".into()))?;
        self.lists[imports].array_fixups[a].0[3] = at;
        Ok(i)
    }

    /// Points sampler `sampler` of `PMaterial` `material` at import `index`.
    ///
    /// # Errors
    /// Fails if the material has no such sampler.
    pub fn set_texture_import(&mut self, material: u32, sampler: &str, index: u32) -> Result<()> {
        let (buffer, slot, old) = self.sampler(material, sampler)?;
        self.point_sampler(buffer, slot, old, index)
    }

    /// A material's sampler: (its buffer list, the texture reference's
    /// pointer fixup, the user fixup it uses).
    fn sampler(&self, material: u32, sampler: &str) -> Result<(usize, usize, u32)> {
        let materials = self.list_named("PMaterial")?;
        let field = self.offset_of(materials, "m_parameterBuffer")?;
        let k = self
            .pointer_at(materials, material, field)
            .ok_or(Error::Unsupported(
                "material without a parameter buffer".into(),
            ))?;
        let buffer = self.lists[materials].pointer_fixups[k].target_list() as usize;
        let defs_field = self.offset_of(buffer, "m_tweakableShaderParameterDefinitions")?;
        let k = self
            .pointer_at(buffer, 0, defs_field + 4)
            .ok_or(Error::Unsupported("buffer without definitions".into()))?;
        let defs = self.lists[buffer].pointer_fixups[k];
        let dl = defs.target_list() as usize;
        let (o_name, o_type, o_loc) = (
            self.offset_of(dl, "m_name")?,
            self.offset_of(dl, "m_parameterType")? as usize,
            self.offset_of(dl, "m_bufferLoc")? as usize,
        );
        let def = (defs.target()..defs.target() + defs.count())
            .find(|&d| {
                self.lists[dl].object(d)[o_type] == 0x42
                    && self.array_at(dl, d, o_name).is_some_and(|a| {
                        self.array_string(dl, self.lists[dl].array_fixups[a].0[3]) == sampler
                    })
            })
            .ok_or_else(|| Error::Unsupported(format!("material has no sampler {sampler}")))?;
        let loc = u32::from_le_bytes(
            self.lists[dl].object(def)[o_loc..o_loc + 4]
                .try_into()
                .expect("4 bytes"),
        ) & 0xffff;
        let slot = self
            .pointer_at(buffer, 0, 8 + loc + 4)
            .ok_or(Error::Unsupported(
                "sampler without a texture reference".into(),
            ))?;
        let old =
            self.lists[buffer].pointer_fixups[slot]
                .user_fixup()
                .ok_or(Error::Unsupported(
                    "sampler texture is not a user fixup".into(),
                ))?;
        Ok((buffer, slot, old))
    }

    /// Points a sampler's texture reference (pointer fixup `slot` of
    /// `buffer`, now using user fixup `old`) at import `index`.
    fn point_sampler(&mut self, buffer: usize, slot: usize, old: u32, index: u32) -> Result<()> {
        let index = u16::try_from(index).map_err(|_| Error::Unsupported("imports".into()))?;
        let at = u32::try_from(self.user_data.len()).expect("user data");
        self.user_data.extend_from_slice(&index.to_be_bytes());
        let kind = self.user_records[old as usize][0];
        self.user_records.push([kind, 2, at]);
        let record = u32::try_from(self.user_records.len() - 1).expect("user fixups");
        self.lists[buffer].pointer_fixups[slot].0[6] = record;
        Ok(())
    }
}

impl ClusterEdit {
    /// The `PMaterial` segment `segment` draws with: element
    /// `m_materialIndex` of its mesh's material set.
    ///
    /// # Errors
    /// Fails on segments outside a mesh or an index outside the set.
    pub fn segment_material(&self, segment: u32) -> Result<u32> {
        let segs = self.list_named("PMeshSegment")?;
        let meshes = self.list_named("PMesh")?;
        let o = self.offset_of(segs, "m_materialIndex")? as usize;
        let index = u32::from_le_bytes(
            self.lists[segs].object(segment)[o..o + 4]
                .try_into()
                .expect("4 bytes"),
        );
        let (mesh, _) = self.mesh_of(segment)?;
        let set = self.offset_of(meshes, "m_defaultMaterials")?;
        self.lists[meshes]
            .pointer_fixups
            .iter()
            .find(|f| {
                f.object() == mesh
                    && field_offset(&self.cluster, f.member()) == Some(set + 4)
                    && f.count() == index
            })
            .map(PointerFixup::target)
            .ok_or(Error::Unsupported("material index outside the set".into()))
    }

    /// Sets segment `segment`'s `m_materialIndex` (an index into its mesh's
    /// material set, as [`ClusterEdit::add_material`] returns).
    ///
    /// # Errors
    /// Fails on clusters without segments.
    pub fn set_segment_material(&mut self, segment: u32, index: u32) -> Result<()> {
        let segs = self.list_named("PMeshSegment")?;
        let o = self.offset_of(segs, "m_materialIndex")? as usize;
        self.lists[segs].object_mut(segment)[o..o + 4].copy_from_slice(&index.to_le_bytes());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leb_and_mode_5_round_trip() {
        let recs = [
            PointerFixup([3, 0, 7, 2, 0x30, 1, u32::MAX]),
            PointerFixup([3, 4, 300, 2, 0, 12, u32::MAX]),
            PointerFixup([0x8000_0034, 2, 0, 0, 0, 0, 5]),
        ];
        let packed = pack_pointer_fixups(&[(&recs, 600), (&[], 3)]).unwrap();
        let (back, used) =
            crate::fixup::unpack_pointer_fixups(&packed, &[(3, 600), (0, 3)]).unwrap();
        assert_eq!((back.as_slice(), used), (&recs[..], packed.len()));

        let one = [PointerFixup([9, 0, 1, 1, 0, 1, u32::MAX])];
        let packed = pack_pointer_fixups(&[(&one, 1)]).unwrap();
        let (back, _) = crate::fixup::unpack_pointer_fixups(&packed, &[(1, 1)]).unwrap();
        assert_eq!(back, one);

        let arrays = [ArrayFixup([0x8000_0050, 3, 13, 0x200])];
        let packed = pack_array_fixups(&[(&arrays, 5)]).unwrap();
        let (back, _) = unpack_array_fixups(&packed, &[(1, 5)]).unwrap();
        assert_eq!(back, arrays);
    }
}
