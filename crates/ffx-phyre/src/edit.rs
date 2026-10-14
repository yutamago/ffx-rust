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
    /// User fixup data, user fixup records, header class counts and
    /// header class fixups, as stored.
    tail: Vec<u8>,
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
            tail: bytes[data_end..cluster.fixups_offset].to_vec(),
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
            let mut header = l.header;
            put(&mut header, 4, l.count() as usize)?;
            put(&mut header, 8, l.objects.len() + l.arrays.len())?;
            put(&mut header, 0xc, l.objects.len())?;
            put(&mut header, 0x10, l.arrays.len())?;
            put(&mut header, 0x18, l.array_fixups.len())?;
            put(&mut header, 0x1c, l.pointer_fixups.len())?;
            put(&mut header, 0x20, l.pointer_array_fixups.len())?;
            out.extend_from_slice(&header);
            data.extend_from_slice(&l.objects);
            data.extend_from_slice(&l.arrays);
        }
        let total_data = data.len();
        out.extend(data);
        out.extend_from_slice(&self.tail);

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
            (14, total_data),
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
    fn mesh_of(&self, segment: u32) -> Result<(u32, usize)> {
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
