//! The editor: a sidebar of the overlay (F11) that changes the look per
//! scene or for all scenes — the sun, point and spot lights, the lights
//! found in glows, materials and textures (picked by clicking in the
//! scene) — and keeps the changes in files (`edits.rs`).

use std::path::PathBuf;

use crate::edits::{Edits, Part, SceneEdits, Status};
use crate::enhanced::Picked;
use crate::enhanced::lights::{GlowEdit, Kind as LightKind, Light, Sun};
use crate::enhanced::materials::{Kind as MaterialKind, Material};

/// The egui texture the picked texture's preview is drawn with.
pub const PREVIEW: egui::TextureId = egui::TextureId::User(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Tab {
    #[default]
    Light,
    Materials,
    Files,
}

/// Where changes go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Scope {
    #[default]
    Scene,
    Global,
}

/// What a click in the scene does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Mode {
    /// Picks the texture there.
    #[default]
    Select,
    /// Puts a new light there.
    Place(LightKind),
}

/// A file being taken in: its changes and, per part, how it stands and
/// whether to take it.
struct Import {
    file: String,
    edits: Edits,
    parts: Vec<(Part, Status, bool)>,
}

/// What the editor sees of the renderer and asks of it, each frame.
pub struct Io<'a> {
    pub scene: &'a str,
    /// The settings profiles' names.
    pub profiles: &'a [String],
    pub camera: Option<([f32; 3], [f32; 3])>,
    /// The lights found in glows (before the editor's changes).
    pub glows: &'a [Light],
    /// A pick the renderer has done since the last frame.
    pub picked: Option<Picked>,
    /// Out: a pick to do (0–1 of the screen).
    pub pick: Option<(f32, f32)>,
    /// The texture whose draws are outlined.
    pub selected: Option<u64>,
    /// The picked texture's preview is available as [`PREVIEW`].
    pub preview: bool,
}

pub struct Editor {
    pub open: bool,
    pub edits: Edits,
    /// As last saved or loaded.
    saved: Edits,
    path: PathBuf,
    dir: PathBuf,
    textures: PathBuf,
    tab: Tab,
    scope: Scope,
    mode: Mode,
    light: Option<usize>,
    /// The texture being edited, and what the pick said of it.
    texture: Option<(u64, bool)>,
    pub message: String,
    import: Option<Import>,
    export_name: String,
    files: Vec<String>,
    images: Vec<String>,
    /// The scene and edits the renderer has (to apply only changes).
    applied: Option<(String, Edits)>,
}

fn files_in(dir: &std::path::Path, ext: &str) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .map(|d| {
            d.filter_map(|e| {
                let name = e.ok()?.file_name().into_string().ok()?;
                std::path::Path::new(&name)
                    .extension()
                    .is_some_and(|x| x.eq_ignore_ascii_case(ext))
                    .then_some(name)
            })
            .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// Azimuth and elevation (degrees) of a direction, y up.
fn angles(d: [f32; 3]) -> (f32, f32) {
    let d = crate::enhanced::math::normalize(d);
    (
        d[0].atan2(d[2]).to_degrees(),
        d[1].clamp(-1.0, 1.0).asin().to_degrees(),
    )
}

fn direction(azimuth: f32, elevation: f32) -> [f32; 3] {
    let (a, e) = (azimuth.to_radians(), elevation.to_radians());
    [a.sin() * e.cos(), e.sin(), a.cos() * e.cos()]
}

/// A direction as two sliders.
fn direction_ui(ui: &mut egui::Ui, d: &mut [f32; 3]) {
    let (mut az, mut el) = angles(*d);
    let before = (az, el);
    ui.add(egui::Slider::new(&mut az, -180.0..=180.0).text("turn (°)"));
    ui.add(egui::Slider::new(&mut el, -90.0..=90.0).text("up/down (°)"));
    if (az, el) != before {
        *d = direction(az, el);
    }
}

/// An optional value: a box turns it on (from `default`), the widget sets
/// it.
fn optional<T: Clone>(
    ui: &mut egui::Ui,
    v: &mut Option<T>,
    label: &str,
    default: T,
    add: impl FnOnce(&mut egui::Ui, &mut T),
) {
    ui.horizontal(|ui| {
        let mut on = v.is_some();
        if ui.checkbox(&mut on, label).changed() {
            *v = on.then(|| default.clone());
        }
        if let Some(x) = v.as_mut() {
            add(ui, x);
        }
    });
}

impl Editor {
    /// The editor of `edits.toml` beside `exe_dir`'s renderer.
    #[must_use]
    pub fn new(exe_dir: &std::path::Path) -> Self {
        let path = exe_dir.join("edits.toml");
        let (edits, error) = Edits::load(&path);
        Self {
            open: false,
            saved: edits.clone(),
            edits,
            path,
            dir: exe_dir.join("edits"),
            textures: exe_dir.join("textures"),
            tab: Tab::Light,
            scope: Scope::Scene,
            mode: Mode::Select,
            light: None,
            texture: None,
            message: error.unwrap_or_default(),
            import: None,
            export_name: "my edits".into(),
            files: Vec::new(),
            images: Vec::new(),
            applied: None,
        }
    }

    /// Whether the edits for `scene` changed since the renderer last got
    /// them (and notes that it has them now).
    pub fn changed(&mut self, scene: &str) -> bool {
        let now = Some((scene.to_owned(), self.edits.clone()));
        if self.applied == now {
            return false;
        }
        self.applied = now;
        true
    }

    fn target<'a>(edits: &'a mut Edits, scope: Scope, scene: &str) -> &'a mut SceneEdits {
        match scope {
            Scope::Global => &mut edits.global,
            Scope::Scene => edits.scene_mut(scene),
        }
    }

    /// The panel. `click`: the pointer clicked the scene (not a panel) at
    /// this place (0–1 of the screen).
    pub fn ui(&mut self, root: &mut egui::Ui, io: &mut Io, click: Option<(f32, f32)>) {
        if !self.open {
            io.selected = None;
            return;
        }
        if let Some(at) = click {
            io.pick = Some(at);
        }
        if let Some(p) = io.picked.take() {
            self.picked(&p, io);
        }
        io.selected = self
            .texture
            .map(|t| t.0)
            .filter(|_| self.tab == Tab::Materials);
        egui::Panel::right("editor")
            .default_size(380.0)
            .resizable(true)
            .show(root, |ui| {
                ui.heading("Editor");
                ui.horizontal(|ui| {
                    ui.label("Scene");
                    ui.monospace(if io.scene.is_empty() { "?" } else { io.scene });
                    if !io.scene.is_empty() {
                        let label = &mut self.edits.scene_mut(io.scene).label;
                        ui.add(
                            egui::TextEdit::singleline(label)
                                .hint_text("a name for it")
                                .desired_width(150.0),
                        );
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Changes go to");
                    ui.selectable_value(&mut self.scope, Scope::Scene, "this scene");
                    ui.selectable_value(&mut self.scope, Scope::Global, "all scenes");
                });
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.tab, Tab::Light, "Light");
                    ui.selectable_value(&mut self.tab, Tab::Materials, "Materials");
                    if ui
                        .selectable_value(&mut self.tab, Tab::Files, "Files")
                        .clicked()
                    {
                        self.files = files_in(&self.dir, "toml");
                    }
                    let dirty = self.edits != self.saved;
                    if ui
                        .add_enabled(dirty, egui::Button::new("Save"))
                        .on_hover_text("Write the changes to edits.toml")
                        .clicked()
                    {
                        self.save();
                    }
                });
                if !self.message.is_empty() {
                    ui.label(&self.message);
                }
                ui.separator();
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.tab {
                        Tab::Light => self.light_ui(ui, io),
                        Tab::Materials => self.material_ui(ui, io),
                        Tab::Files => self.files_ui(ui),
                    });
            });
    }

    fn save(&mut self) {
        self.message = match self.edits.save(&self.path) {
            Ok(()) => {
                self.saved = self.edits.clone();
                "saved edits.toml".into()
            }
            Err(e) => format!("not saved: {e}"),
        };
    }

    /// A pick came back: a light goes there, or its texture is the one
    /// to edit.
    fn picked(&mut self, p: &Picked, io: &Io) {
        match self.mode {
            Mode::Place(kind) => {
                self.mode = Mode::Select;
                if p.empty {
                    self.message = "nothing there to put a light on".into();
                    return;
                }
                let n = p.normal;
                let away = if kind == LightKind::Spot { 25.0 } else { 6.0 };
                let light = Light {
                    name: format!(
                        "{} {}",
                        if kind == LightKind::Spot {
                            "Spot"
                        } else {
                            "Light"
                        },
                        self.scope_lights(io.scene).len() + 1
                    ),
                    kind,
                    position: [
                        p.position[0] + n[0] * away,
                        p.position[1] + n[1] * away,
                        p.position[2] + n[2] * away,
                    ],
                    direction: n.map(|v| -v),
                    ..Light::default()
                };
                let lights = &mut Self::target(&mut self.edits, self.scope, io.scene).lights;
                lights.push(light);
                self.light = Some(lights.len() - 1);
                self.message = "light added".into();
            }
            Mode::Select => {
                if p.texture == 0 {
                    self.message = if p.empty {
                        "nothing enhanced there (sky, backdrop or effect)".into()
                    } else {
                        "that has no texture".into()
                    };
                    return;
                }
                self.texture = Some((p.texture, p.character));
                self.tab = Tab::Materials;
                self.images = files_in(&self.textures, "png");
                self.message.clear();
            }
        }
    }

    fn scope_lights(&self, scene: &str) -> &[Light] {
        match self.scope {
            Scope::Global => &self.edits.global.lights,
            Scope::Scene => self.edits.scenes.get(scene).map_or(&[], |s| &s.lights),
        }
    }

    #[allow(clippy::too_many_lines)] // the sun, the lights, the glows
    fn light_ui(&mut self, ui: &mut egui::Ui, io: &mut Io) {
        let scope = self.scope;
        let scene = io.scene.to_owned();
        let look_at = io.camera.map(|c| c.1);
        let target = Self::target(&mut self.edits, scope, &scene);
        ui.horizontal(|ui| {
            ui.label("Settings profile");
            let none = match scope {
                Scope::Scene => "none of its own",
                Scope::Global => "the one chosen by hand",
            };
            let shown = if target.profile.is_empty() {
                none.to_owned()
            } else {
                target.profile.clone()
            };
            egui::ComboBox::from_id_salt("scene profile")
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut target.profile, String::new(), none);
                    for n in io.profiles {
                        ui.selectable_value(&mut target.profile, n.clone(), n);
                    }
                });
        })
        .response
        .on_hover_text(match scope {
            Scope::Scene => "Entering this scene switches to this profile",
            Scope::Global => "Scenes without a profile of their own switch to this one",
        });
        egui::CollapsingHeader::new("Sun (the key light)")
            .default_open(true)
            .show(ui, |ui| {
                let sun: &mut Sun = &mut Self::target(&mut self.edits, scope, &scene).sun;
                optional(
                    ui,
                    &mut sun.direction,
                    "direction",
                    [-0.35, -0.85, 0.4],
                    |ui, d| {
                        ui.vertical(|ui| direction_ui(ui, d));
                    },
                );
                optional(ui, &mut sun.color, "colour", [1.0, 0.95, 0.85], |ui, c| {
                    ui.color_edit_button_rgb(c);
                });
                optional(ui, &mut sun.scale, "brightness ×", 1.0, |ui, s| {
                    ui.add(egui::Slider::new(s, 0.0..=4.0));
                });
            });
        egui::CollapsingHeader::new("Lights")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .button("Add point light")
                        .on_hover_text("Then click where it goes")
                        .clicked()
                    {
                        self.mode = Mode::Place(LightKind::Point);
                        self.message = "click in the scene where the light goes".into();
                    }
                    if ui
                        .button("Add spot light")
                        .on_hover_text("Then click what it lights")
                        .clicked()
                    {
                        self.mode = Mode::Place(LightKind::Spot);
                        self.message = "click what the spot light shines on".into();
                    }
                });
                let lights = &mut Self::target(&mut self.edits, scope, &scene).lights;
                let mut remove = None;
                let mut copy = None;
                for (k, l) in lights.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut l.enabled, "");
                        let text = format!(
                            "{} ({})",
                            l.name,
                            if l.kind == LightKind::Spot {
                                "spot"
                            } else {
                                "point"
                            }
                        );
                        if ui.selectable_label(self.light == Some(k), text).clicked() {
                            self.light = (self.light != Some(k)).then_some(k);
                        }
                    });
                    if self.light != Some(k) {
                        continue;
                    }
                    ui.indent(k, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("name");
                            ui.text_edit_singleline(&mut l.name);
                        });
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut l.kind, LightKind::Point, "point");
                            ui.selectable_value(&mut l.kind, LightKind::Spot, "spot");
                        });
                        ui.horizontal(|ui| {
                            ui.label("position");
                            for v in &mut l.position {
                                ui.add(egui::DragValue::new(v).speed(0.5));
                            }
                        });
                        if let Some(t) = look_at
                            && ui.button("To what the camera looks at").clicked()
                        {
                            l.position = [t[0], t[1] + 8.0, t[2]];
                        }
                        ui.horizontal(|ui| {
                            ui.label("colour");
                            ui.color_edit_button_rgb(&mut l.color);
                        });
                        ui.add(
                            egui::Slider::new(&mut l.intensity, 0.0..=20.0)
                                .logarithmic(true)
                                .text("brightness"),
                        );
                        ui.add(egui::Slider::new(&mut l.range, 5.0..=500.0).text("range"));
                        if l.kind == LightKind::Spot {
                            direction_ui(ui, &mut l.direction);
                            ui.add(egui::Slider::new(&mut l.outer, 5.0..=170.0).text("cone (°)"));
                            l.inner = l.inner.min(l.outer);
                            ui.add(
                                egui::Slider::new(&mut l.inner, 0.0..=l.outer).text("full in (°)"),
                            );
                        }
                        ui.checkbox(&mut l.shadows, "Shadows (ray traced)");
                        ui.add(egui::Slider::new(&mut l.specular, 0.0..=2.0).text("highlights"));
                        ui.horizontal(|ui| {
                            if ui.button("Duplicate").clicked() {
                                copy = Some(k);
                            }
                            if ui.button("Delete").clicked() {
                                remove = Some(k);
                            }
                        });
                    });
                }
                if let Some(k) = copy {
                    let mut l = lights[k].clone();
                    l.name.push_str(" copy");
                    l.position[0] += 5.0;
                    lights.push(l);
                    self.light = Some(lights.len() - 1);
                }
                if let Some(k) = remove {
                    lights.remove(k);
                    self.light = None;
                }
                if lights.is_empty() {
                    ui.label("No lights here yet.");
                }
            });
        egui::CollapsingHeader::new("Glows (found lights)").show(ui, |ui| {
            if io.glows.is_empty() {
                ui.label("No glowing effects seen in this scene.");
            }
            // (glow, on, brightness, make it editable) asked for.
            let mut asked = None;
            let edits = &Self::target(&mut self.edits, scope, &scene).glows;
            for (k, g) in io.glows.iter().enumerate() {
                let edit = crate::enhanced::lights::glow_edit(edits, g.position);
                let mut on = edit.is_none_or(|e| e.enabled);
                let mut b = edit.map_or(1.0, |e| e.brightness);
                let label = format!(
                    "{} at ({:.0}, {:.0}, {:.0})",
                    g.name, g.position[0], g.position[1], g.position[2]
                );
                ui.push_id(k, |ui| {
                    let mut changed = ui.checkbox(&mut on, label).changed();
                    ui.horizontal(|ui| {
                        changed |= ui
                            .add(egui::Slider::new(&mut b, 0.0..=4.0).text("brightness ×"))
                            .changed();
                        if ui
                            .button("Make editable")
                            .on_hover_text("A light of your own in its place")
                            .clicked()
                        {
                            asked = Some((k, false, b, true));
                        }
                    });
                    if changed && asked.is_none() {
                        asked = Some((k, on, b, false));
                    }
                });
            }
            if let Some((k, on, b, adopt)) = asked {
                let g = io.glows[k].clone();
                let target = Self::target(&mut self.edits, scope, &scene);
                let near = target.glows.iter().position(|e| {
                    (0..3)
                        .map(|i| (e.position[i] - g.position[i]).powi(2))
                        .sum::<f32>()
                        < crate::enhanced::lights::GLOW_MATCH.powi(2)
                });
                let i = near.unwrap_or_else(|| {
                    target.glows.push(GlowEdit {
                        position: g.position,
                        ..GlowEdit::default()
                    });
                    target.glows.len() - 1
                });
                target.glows[i].enabled = on;
                target.glows[i].brightness = b;
                if adopt {
                    let mut l = g;
                    l.enabled = true;
                    l.intensity *= b;
                    target.lights.push(l);
                    self.light = Some(target.lights.len() - 1);
                }
            }
        });
    }

    #[allow(clippy::too_many_lines)] // the picked texture's material, then the list
    fn material_ui(&mut self, ui: &mut egui::Ui, io: &mut Io) {
        ui.label("Click on something in the scene to edit its material.");
        let scope = self.scope;
        let scene = io.scene.to_owned();
        let Some((hash, character)) = self.texture else {
            self.material_list(ui, &scene);
            return;
        };
        ui.horizontal(|ui| {
            ui.monospace(crate::edits::key(hash));
            ui.label(if character {
                "(character)"
            } else {
                "(scenery)"
            });
            if ui.button("Done").clicked() {
                self.texture = None;
            }
        });
        if io.preview {
            ui.image(egui::load::SizedTexture::new(
                PREVIEW,
                egui::vec2(128.0, 128.0),
            ));
        }
        let key = crate::edits::key(hash);
        let materials = &mut Self::target(&mut self.edits, scope, &scene).materials;
        let mut m = materials.get(&key).cloned().unwrap_or_default();
        let before = m.clone();
        egui::ComboBox::from_label("made of")
            .selected_text(m.kind.name())
            .show_ui(ui, |ui| {
                for k in MaterialKind::ALL {
                    ui.selectable_value(&mut m.kind, k, k.name());
                }
            });
        optional(ui, &mut m.roughness, "roughness", 0.6, |ui, v| {
            ui.add(egui::Slider::new(v, 0.05..=1.0));
        });
        optional(ui, &mut m.metallic, "metal", 1.0, |ui, v| {
            ui.add(egui::Slider::new(v, 0.0..=1.0));
        });
        optional(ui, &mut m.specular, "highlights", 1.0, |ui, v| {
            ui.add(egui::Slider::new(v, 0.0..=2.0));
        });
        optional(ui, &mut m.detail, "surface detail", 1.0, |ui, v| {
            ui.add(egui::Slider::new(v, 0.0..=3.0));
        });
        optional(ui, &mut m.emission, "glow", 1.0, |ui, v| {
            ui.add(egui::Slider::new(v, 0.0..=8.0));
        });
        optional(ui, &mut m.tint, "tint", [1.0, 1.0, 1.0], |ui, c| {
            ui.color_edit_button_rgb(c);
        });
        optional(ui, &mut m.skin, "skin by colour", true, |ui, v| {
            ui.checkbox(v, "");
        });
        ui.horizontal(|ui| {
            let mut on = m.texture.is_some();
            if ui.checkbox(&mut on, "image instead").changed() {
                m.texture = on.then(|| self.images.first().cloned().unwrap_or_default());
            }
            if let Some(f) = m.texture.as_mut() {
                let shown = if f.is_empty() {
                    "(textures/*.png)".to_owned()
                } else {
                    f.clone()
                };
                egui::ComboBox::from_id_salt("image")
                    .selected_text(shown)
                    .show_ui(ui, |ui| {
                        for name in &self.images {
                            ui.selectable_value(f, name.clone(), name);
                        }
                    });
                if ui
                    .small_button("↻")
                    .on_hover_text("Look for images again")
                    .clicked()
                {
                    self.images = files_in(&self.textures, "png");
                }
            }
        });
        if m != before {
            if m == Material::default() {
                materials.remove(&key);
            } else {
                materials.insert(key.clone(), m);
            }
        }
        if materials.contains_key(&key) && ui.button("Back to automatic").clicked() {
            materials.remove(&key);
        }
        ui.separator();
        self.material_list(ui, &scene);
    }

    /// The materials changed in the scope: click one to edit it.
    fn material_list(&mut self, ui: &mut egui::Ui, scene: &str) {
        let list: Vec<(String, MaterialKind)> = Self::target(&mut self.edits, self.scope, scene)
            .materials
            .iter()
            .map(|(k, m)| (k.clone(), m.kind))
            .collect();
        ui.label(format!("{} materials changed here", list.len()));
        for (k, kind) in list {
            let hash = u64::from_str_radix(&k, 16).unwrap_or(0);
            let on = self.texture.is_some_and(|t| t.0 == hash);
            if ui
                .selectable_label(on, format!("{k} {}", kind.name()))
                .clicked()
            {
                self.texture = Some((hash, false));
                self.images = files_in(&self.textures, "png");
            }
        }
    }

    #[allow(clippy::too_many_lines)] // save, export, the files, what one changes
    fn files_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Save").clicked() {
                self.save();
            }
            if ui
                .button("Revert")
                .on_hover_text("Back to edits.toml as saved")
                .clicked()
            {
                self.edits = self.saved.clone();
                self.message = "back to the saved edits".into();
            }
        });
        ui.separator();
        ui.label("Export everything to edits/<name>.toml:");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.export_name);
            if ui.button("Export").clicked() {
                let name = self
                    .export_name
                    .trim()
                    .chars()
                    .filter(|c| !"/\\:*?\"<>|".contains(*c))
                    .collect::<String>();
                let path = self.dir.join(format!("{name}.toml"));
                self.message = match self.edits.save(&path) {
                    Ok(()) => format!("wrote edits/{name}.toml"),
                    Err(e) => format!("not written: {e}"),
                };
                self.files = files_in(&self.dir, "toml");
            }
        });
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Take in from edits/:");
            if ui.small_button("↻").clicked() {
                self.files = files_in(&self.dir, "toml");
            }
        });
        if self.files.is_empty() {
            ui.label("(no files in edits/)");
        }
        for f in self.files.clone() {
            if ui
                .selectable_label(self.import.as_ref().is_some_and(|i| i.file == f), &f)
                .clicked()
            {
                let (other, err) = Edits::load(&self.dir.join(&f));
                if let Some(e) = err {
                    self.message = e;
                    continue;
                }
                let parts = self
                    .edits
                    .compare(&other)
                    .into_iter()
                    .map(|(p, s)| (p, s, s != Status::Same))
                    .collect();
                self.import = Some(Import {
                    file: f,
                    edits: other,
                    parts,
                });
            }
        }
        let mut take = false;
        if let Some(i) = self.import.as_mut() {
            ui.separator();
            ui.label(format!("{} changes:", i.file));
            if i.parts.is_empty() {
                ui.label("(nothing)");
            }
            for (p, s, on) in &mut i.parts {
                let status = match s {
                    Status::New => "new here",
                    Status::Differs => "differs from yours",
                    Status::Same => "same as yours",
                };
                let label = match p {
                    Part::Global => "all scenes".to_owned(),
                    Part::Scene(k) => {
                        let name = &i.edits.scenes[k].label;
                        if name.is_empty() {
                            k.clone()
                        } else {
                            format!("{k} ({name})")
                        }
                    }
                };
                ui.checkbox(on, format!("{label}: {status}"));
            }
            take = ui.button("Take the ticked ones").clicked();
        }
        if take && let Some(i) = self.import.take() {
            let parts: Vec<Part> = i
                .parts
                .iter()
                .filter(|p| p.2)
                .map(|p| p.0.clone())
                .collect();
            self.edits.take(&i.edits, &parts);
            self.message = format!("took {} parts from {} (not saved yet)", parts.len(), i.file);
        }
    }

    /// Whether a click should pick (the editor is open).
    #[must_use]
    pub fn picking(&self) -> bool {
        self.open
    }
}
