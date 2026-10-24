//! Constant buffers decoded by the shader's own reflection (`RDEF`,
//! [`ffx_dxbc::Reflection`]): only the variables the shader reads
//! (`D3D_SVF_USED`) count, word by word, named after the variable. A buffer
//! the shader does not bind is not compared; a shader without reflection
//! has its whole window compared, as `cb<slot>`.

use std::sync::Arc;

use ffx_dxbc::{Member, Reflection};

/// The used constants a stage saw in one slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Constants {
    /// `vs` or `ps`.
    pub stage: &'static str,
    pub slot: u8,
    pub vars: Vec<Var>,
}

/// One used variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Var {
    pub name: Arc<str>,
    /// Byte offset in the window.
    pub offset: u32,
    /// Its words: (bits, is a float); `None` past the end of the window.
    pub words: Vec<(Option<u32>, bool)>,
    /// A struct's members, for the names of its words.
    pub members: Arc<[Member]>,
}

/// The member holding byte `rel` of a struct: its path, the offset in
/// it, and whether it is a single scalar.
fn member_at(members: &[Member], rel: u32) -> Option<(String, u32, bool)> {
    let m = members.iter().rev().find(|m| m.offset <= rel)?;
    let sub = rel - m.offset;
    if let Some((path, at, scalar)) = member_at(&m.members, sub) {
        return Some((format!("{}.{path}", m.name), at, scalar));
    }
    let scalar = m.rows.max(1) * m.columns.max(1) == 1 && m.elements == 0;
    Some((m.name.clone(), sub, scalar))
}

fn register(sub: u32) -> String {
    let comp = ["x", "y", "z", "w"][(sub % 16 / 4) as usize];
    format!("[{}].{comp}", sub / 16)
}

impl Var {
    /// The report name of word `i`: `Var` for a scalar, else
    /// `Var[register].component` (registers counted from the variable).
    #[must_use]
    pub fn word_name(&self, i: usize) -> String {
        let rel = u32::try_from(i * 4).unwrap_or(u32::MAX);
        if let Some((path, sub, scalar)) = member_at(&self.members, rel) {
            let suffix = if scalar && sub == 0 {
                String::new()
            } else {
                register(sub)
            };
            return format!("{}.{path}{suffix}", self.name);
        }
        if self.words.len() == 1 {
            return self.name.to_string();
        }
        format!("{}{}", self.name, register(rel))
    }
}

/// A shader's reflection as needed here: per constant buffer slot, the used
/// variables (name, offset, size, word types).
#[derive(Debug, Clone, Default)]
pub struct Layout {
    /// `None`: the shader has no reflection, so every slot is compared
    /// whole.
    pub slots: Option<Vec<(u32, Vec<VarDef>)>>,
    /// Texture and sampler registers the shader binds: (first, count).
    pub textures: Option<Vec<(u32, u32)>>,
    pub samplers: Option<Vec<(u32, u32)>>,
}

#[derive(Debug, Clone)]
pub struct VarDef {
    pub name: Arc<str>,
    pub offset: u32,
    pub size: u32,
    /// Whether each word is a float.
    pub floats: Vec<bool>,
    pub members: Arc<[Member]>,
}

fn member_float(members: &[Member], rel: u32) -> Option<bool> {
    let m = members.iter().rev().find(|m| m.offset <= rel)?;
    if m.members.is_empty() {
        Some(m.base == 3)
    } else {
        member_float(&m.members, rel - m.offset).or(Some(m.base == 3))
    }
}

impl Layout {
    /// From a shader's bytecode.
    #[must_use]
    pub fn of(bytecode: &[u8]) -> Self {
        let Some(r) = ffx_dxbc::Container::parse(bytecode)
            .ok()
            .and_then(|c| c.reflection().ok().flatten())
        else {
            return Self::default();
        };
        Self::from_reflection(&r)
    }

    #[must_use]
    pub fn from_reflection(r: &Reflection) -> Self {
        use ffx_dxbc::BindKind;
        let mut slots = Vec::new();
        for b in &r.bindings {
            if b.kind != BindKind::ConstantBuffer {
                continue;
            }
            let Some(cb) = r.buffers.iter().find(|c| c.name == b.name) else {
                continue;
            };
            let vars = cb
                .variables
                .iter()
                .filter(|v| v.used && v.size > 0)
                .map(|v| VarDef {
                    name: Arc::from(v.name.as_str()),
                    offset: v.offset,
                    size: v.size,
                    floats: (0..v.size.div_ceil(4))
                        .map(|w| {
                            if v.members.is_empty() {
                                v.base == 3
                            } else {
                                member_float(&v.members, w * 4).unwrap_or(false)
                            }
                        })
                        .collect(),
                    members: Arc::from(v.members.as_slice()),
                })
                .collect();
            slots.push((b.slot, vars));
        }
        let ranges = |kinds: &[BindKind]| {
            r.bindings
                .iter()
                .filter(|b| kinds.contains(&b.kind))
                .map(|b| (b.slot, b.count.max(1)))
                .collect()
        };
        Self {
            slots: Some(slots),
            textures: Some(ranges(&[BindKind::Texture, BindKind::TextureBuffer])),
            samplers: Some(ranges(&[BindKind::Sampler])),
        }
    }

    /// Whether the shader reads texture register `slot` (true without
    /// reflection).
    #[must_use]
    pub fn uses_texture(&self, slot: u8) -> bool {
        uses(self.textures.as_deref(), slot)
    }

    #[must_use]
    pub fn uses_sampler(&self, slot: u8) -> bool {
        uses(self.samplers.as_deref(), slot)
    }

    /// The used constants of the window bound at `slot`, or `None` when
    /// the shader does not bind that slot.
    #[must_use]
    pub fn decode(&self, stage: &'static str, slot: u8, window: &[u8]) -> Option<Constants> {
        let word = |at: u32| -> Option<u32> {
            let at = at as usize;
            window
                .get(at..at + 4)
                .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        };
        let vars = match &self.slots {
            None => {
                let n = u32::try_from(window.len().div_ceil(4)).unwrap_or(u32::MAX);
                vec![Var {
                    name: Arc::from(format!("cb{slot}").as_str()),
                    offset: 0,
                    words: (0..n).map(|i| (word(i * 4), true)).collect(),
                    members: Arc::from(Vec::new()),
                }]
            }
            Some(slots) => {
                let (_, defs) = slots.iter().find(|(s, _)| *s == u32::from(slot))?;
                defs.iter()
                    .map(|d| Var {
                        name: d.name.clone(),
                        offset: d.offset,
                        words: d
                            .floats
                            .iter()
                            .zip(0u32..)
                            .map(|(&f, i)| (word(d.offset + i * 4), f))
                            .collect(),
                        members: d.members.clone(),
                    })
                    .collect()
            }
        };
        Some(Constants { stage, slot, vars })
    }
}

fn uses(ranges: Option<&[(u32, u32)]>, slot: u8) -> bool {
    ranges.is_none_or(|r| {
        r.iter()
            .any(|&(first, n)| (first..first + n).contains(&u32::from(slot)))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> Layout {
        Layout {
            slots: Some(vec![(
                2,
                vec![VarDef {
                    name: Arc::from("MaterialColour"),
                    offset: 16,
                    size: 16,
                    floats: vec![true; 4],
                    members: Arc::from(Vec::new()),
                }],
            )]),
            textures: Some(vec![(0, 2)]),
            samplers: Some(vec![(0, 1)]),
        }
    }

    #[test]
    fn only_used_variables_of_bound_slots() {
        let window: Vec<u8> = (0..32u8).collect();
        let l = layout();
        assert!(l.decode("ps", 0, &window).is_none());
        let c = l.decode("ps", 2, &window).unwrap();
        assert_eq!(c.vars.len(), 1);
        assert_eq!(
            c.vars[0].words[0].0,
            Some(u32::from_le_bytes([16, 17, 18, 19]))
        );
        assert_eq!(c.vars[0].word_name(1), "MaterialColour[0].y");
        // A short window: the rest is absent.
        let c = l.decode("ps", 2, &window[..24]).unwrap();
        assert_eq!(c.vars[0].words[2].0, None);
        assert!(l.uses_texture(1) && !l.uses_texture(2));
        assert!(Layout::default().uses_sampler(9));
    }
}
