//! The sequence editor's data: cutscenes recorded as they play (frames
//! kept, compressed, for this session: they draw with the game's objects
//! of now), and per recorded sequence its camera path and depth-of-field
//! keyframes, kept in `sequences.toml` beside the renderer. In play, a
//! scene's camera is matched to its sequences' paths: where it follows one,
//! the keyframes there set the depth of field.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Frames a recording keeps at most (two minutes at 30 a second).
pub const MAX_FRAMES: usize = 3600;
/// How near the camera must be to a recorded one to be on the path: the
/// eye, and the look-at point (world units).
const EYE_MATCH: f32 = 0.5;
const LOOK_MATCH: f32 = 2.0;

/// The depth of field from a keyframe on.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Key {
    pub frame: u32,
    /// Focus distance, world units (0: automatic).
    pub focus: f32,
    pub strength: f32,
    /// The largest blur, pixels of a 1080-line image.
    pub radius: f32,
}

impl Default for Key {
    fn default() -> Self {
        Self {
            frame: 0,
            focus: 50.0,
            strength: 1.0,
            radius: 8.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Sequence {
    pub scene: String,
    pub name: String,
    /// The camera each frame: eye and look-at (render space).
    pub cameras: Vec<[f32; 6]>,
    pub keys: Vec<Key>,
}

impl Sequence {
    /// The depth of field at `frame` (between keys: eased from one to the
    /// next; before the first and after the last: theirs).
    #[must_use]
    pub fn dof_at(&self, frame: f32) -> Option<Key> {
        let mut keys = self.keys.clone();
        keys.sort_by_key(|k| k.frame);
        let first = *keys.first()?;
        let after = keys.iter().position(|k| k.frame as f32 > frame);
        Some(match after {
            None => *keys.last().expect("not empty"),
            Some(0) => first,
            Some(i) => {
                let (a, b) = (keys[i - 1], keys[i]);
                let t =
                    ((frame - a.frame as f32) / (b.frame - a.frame).max(1) as f32).clamp(0.0, 1.0);
                let t = t * t * (3.0 - 2.0 * t);
                let lerp = |x: f32, y: f32| x + (y - x) * t;
                Key {
                    frame: frame as u32,
                    focus: lerp(a.focus, b.focus),
                    strength: lerp(a.strength, b.strength),
                    radius: lerp(a.radius, b.radius),
                }
            }
        })
    }

    fn matches(&self, frame: usize, camera: &([f32; 3], [f32; 3])) -> bool {
        self.cameras.get(frame).is_some_and(|c| {
            let d = |a: &[f32], b: &[f32; 3]| {
                ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
            };
            d(&c[..3], &camera.0) < EYE_MATCH && d(&c[3..], &camera.1) < LOOK_MATCH
        })
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
struct File {
    sequence: Vec<Sequence>,
}

/// A recorded frame: its camera and the frame, compressed.
pub struct Recorded {
    pub camera: Option<([f32; 3], [f32; 3])>,
    data: Vec<u8>,
}

impl Recorded {
    #[must_use]
    pub fn new(frame: &ffx_gfx::Frame, camera: Option<([f32; 3], [f32; 3])>) -> Self {
        Self {
            camera,
            data: lz4_flex::compress_prepend_size(&ffx_gfx::encode_frame(frame)),
        }
    }

    /// The frame again.
    #[must_use]
    pub fn frame(&self) -> Option<ffx_gfx::Frame> {
        let bytes = lz4_flex::decompress_size_prepended(&self.data).ok()?;
        ffx_gfx::decode_frame(&bytes).ok()
    }

    #[must_use]
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

pub struct Sequences {
    pub list: Vec<Sequence>,
    path: PathBuf,
    saved: Vec<Sequence>,
    /// This session's recordings, by sequence (index into `list`).
    pub takes: Vec<(usize, Vec<Recorded>)>,
    /// A recording going on: its scene and frames.
    pub recording: Option<(String, Vec<Recorded>)>,
    /// In play: the sequence and frame the camera follows.
    lock: Option<(usize, usize)>,
    pub message: String,
}

impl Sequences {
    /// The sequences of `sequences.toml` beside `exe_dir`'s renderer.
    #[must_use]
    pub fn new(exe_dir: &Path) -> Self {
        let path = exe_dir.join("sequences.toml");
        let (list, message) = match std::fs::read_to_string(&path) {
            Ok(t) => match toml::from_str::<File>(&t) {
                Ok(f) => (f.sequence, String::new()),
                Err(e) => (Vec::new(), format!("{}: {e}", path.display())),
            },
            Err(_) => (Vec::new(), String::new()),
        };
        Self {
            saved: list.clone(),
            list,
            path,
            takes: Vec::new(),
            recording: None,
            lock: None,
            message,
        }
    }

    #[must_use]
    pub fn dirty(&self) -> bool {
        self.list != self.saved
    }

    /// Writes `sequences.toml`.
    pub fn save(&mut self) {
        let text = toml::to_string(&File {
            sequence: self.list.clone(),
        });
        self.message = match text
            .map_err(|e| e.to_string())
            .and_then(|t| std::fs::write(&self.path, t).map_err(|e| e.to_string()))
        {
            Ok(()) => {
                self.saved = self.list.clone();
                "saved sequences.toml".into()
            }
            Err(e) => format!("not saved: {e}"),
        };
    }

    /// Starts recording in `scene`.
    pub fn record(&mut self, scene: &str) {
        self.recording = Some((scene.to_owned(), Vec::new()));
        self.message = "recording".into();
    }

    /// A frame of the game while recording (stops by itself when the
    /// scene changes or it is full).
    pub fn frame(
        &mut self,
        scene: &str,
        frame: &ffx_gfx::Frame,
        camera: Option<([f32; 3], [f32; 3])>,
    ) {
        let Some((s, frames)) = self.recording.as_mut() else {
            return;
        };
        if s != scene || frames.len() >= MAX_FRAMES {
            self.stop();
            return;
        }
        if frame.cmds.is_empty() {
            return;
        }
        frames.push(Recorded::new(frame, camera));
    }

    /// Ends the recording: a new sequence of it, with this take. Returns
    /// its index.
    pub fn stop(&mut self) -> Option<usize> {
        let (scene, frames) = self.recording.take()?;
        if frames.is_empty() {
            self.message = "nothing recorded".into();
            return None;
        }
        let n = self.list.iter().filter(|s| s.scene == scene).count() + 1;
        let cameras = frames
            .iter()
            .map(|f| {
                f.camera
                    .map_or([0.0; 6], |(e, l)| [e[0], e[1], e[2], l[0], l[1], l[2]])
            })
            .collect();
        let bytes: usize = frames.iter().map(Recorded::size).sum();
        self.message = format!(
            "recorded {} frames ({:.0} MB)",
            frames.len(),
            bytes as f32 / 1e6
        );
        self.list.push(Sequence {
            name: format!("{scene} {n}"),
            scene,
            cameras,
            keys: Vec::new(),
        });
        let k = self.list.len() - 1;
        self.takes.push((k, frames));
        Some(k)
    }

    /// The take of sequence `k` (this session's recording).
    #[must_use]
    pub fn take(&self, k: usize) -> Option<&[Recorded]> {
        self.takes.iter().find(|t| t.0 == k).map(|t| t.1.as_slice())
    }

    /// Deletes sequence `k` (and its take).
    pub fn delete(&mut self, k: usize) {
        if k < self.list.len() {
            self.list.remove(k);
            self.takes.retain(|t| t.0 != k);
            for t in &mut self.takes {
                if t.0 > k {
                    t.0 -= 1;
                }
            }
            self.lock = None;
        }
    }

    /// In play: the depth of field the keyframes give where the camera
    /// follows a sequence of `scene`, and which (sequence, frame).
    pub fn follow(
        &mut self,
        scene: &str,
        camera: Option<([f32; 3], [f32; 3])>,
    ) -> Option<(Key, usize, usize)> {
        let Some(camera) = camera else {
            self.lock = None;
            return None;
        };
        // On a path: the next frames first.
        if let Some((s, f)) = self.lock
            && let Some(seq) = self.list.get(s)
            && seq.scene == scene
            && let Some(next) = (f + 1..f + 4)
                .chain(std::iter::once(f))
                .find(|&g| seq.matches(g, &camera))
        {
            self.lock = Some((s, next));
        } else {
            self.lock = self
                .list
                .iter()
                .enumerate()
                .filter(|(_, s)| s.scene == scene && !s.keys.is_empty())
                .find_map(|(k, s)| {
                    (0..s.cameras.len())
                        .find(|&f| s.matches(f, &camera))
                        .map(|f| (k, f))
                });
        }
        let (s, f) = self.lock?;
        self.list[s].dof_at(f as f32).map(|k| (k, s, f))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_ease_and_paths_follow() {
        let mut seq = Sequence {
            scene: "s".into(),
            name: "a".into(),
            cameras: (0..10)
                .map(|i| [i as f32 * 5.0, 0.0, 0.0, i as f32 * 5.0, 0.0, 10.0])
                .collect(),
            keys: vec![
                Key {
                    frame: 2,
                    focus: 10.0,
                    ..Key::default()
                },
                Key {
                    frame: 6,
                    focus: 30.0,
                    ..Key::default()
                },
            ],
        };
        assert!((seq.dof_at(0.0).unwrap().focus - 10.0).abs() < 1e-5);
        assert!((seq.dof_at(4.0).unwrap().focus - 20.0).abs() < 1e-5);
        assert!((seq.dof_at(9.0).unwrap().focus - 30.0).abs() < 1e-5);
        seq.keys.push(Key::default());
        let mut all = Sequences {
            list: vec![seq],
            path: PathBuf::new(),
            saved: Vec::new(),
            takes: Vec::new(),
            recording: None,
            lock: None,
            message: String::new(),
        };
        let cam = |i: f32| Some(([i * 5.0, 0.0, 0.0], [i * 5.0, 0.0, 10.0]));
        assert_eq!(all.follow("s", cam(3.0)).map(|f| f.2), Some(3));
        assert_eq!(all.follow("s", cam(4.0)).map(|f| f.2), Some(4));
        assert!(all.follow("other", cam(4.0)).is_none());
        assert!(
            all.follow("s", Some(([100.0, 0.0, 0.0], [0.0; 3])))
                .is_none()
        );
    }
}
