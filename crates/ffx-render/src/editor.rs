//! The editor: a sidebar of the overlay (F11) that changes the look per
//! scene or for all scenes — the scene's lights as a list with each
//! light's properties (the default rig, or a scene's own), the lights
//! found in glows, materials and textures (picked by clicking in the
//! scene) — and keeps the changes in files (`edits.rs`).

use std::path::PathBuf;

use crate::edits::{Edits, Part, SceneEdits, Status};
use crate::enhanced::Picked;
use crate::enhanced::lights::{GlowEdit, Light};
use crate::enhanced::materials::{Kind as MaterialKind, Material};
use crate::enhanced::rig::{self, GameLight, LightType, Mobility, Rig, SceneLight};

/// The egui texture the picked texture's preview is drawn with.
pub const PREVIEW: egui::TextureId = egui::TextureId::User(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Tab {
    #[default]
    Light,
    Materials,
    Sequence,
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
    /// Puts a new light there (point or spot).
    Place(LightType),
    /// Sets the selected sequence key's focus to what is there.
    Focus,
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
    /// The game's own lights this frame (what lights that follow it get).
    pub game: Option<rig::Game>,
    /// The game's camera: view-projection (rows) and eye, for the
    /// handles.
    pub view: Option<(crate::enhanced::math::Mat, [f32; 3])>,
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

#[allow(clippy::struct_excessive_bools)] // independent states of the panel
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
    /// The selected light's handles.
    gizmo: crate::gizmo::Gizmo,
    /// The sequence editor: its sequences, the game paused, the frame
    /// shown and whether it plays, the sequence and key picked, and
    /// whether the recording shows while paused.
    pub seq: crate::sequence::Sequences,
    pub paused: bool,
    pub cursor: usize,
    pub playing: bool,
    pub sequence: Option<usize>,
    key: Option<usize>,
    pub preview: bool,
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
    /// Where the selected light is (point and spot lights), for the free
    /// camera to centre on.
    #[must_use]
    pub fn selected_point(&self, scene: &str) -> Option<[f32; 3]> {
        use crate::enhanced::rig::LightType;
        let l = self.edits.rig_for(scene).lights.get(self.light?)?;
        matches!(l.kind, LightType::Point | LightType::Spot).then_some(l.position)
    }

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
            gizmo: crate::gizmo::Gizmo::default(),
            seq: crate::sequence::Sequences::new(exe_dir),
            paused: false,
            cursor: 0,
            playing: false,
            sequence: None,
            key: None,
            preview: true,
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
        // The lights' handles (in the Light tab): a click they take does
        // not pick.
        let click = if self.tab == Tab::Light && self.handles(root.ctx(), io) {
            None
        } else {
            click
        };
        // In the sequence editor a click only picks a focus.
        let click = click.filter(|_| self.tab != Tab::Sequence || self.mode == Mode::Focus);
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
                    ui.selectable_value(&mut self.tab, Tab::Sequence, "Sequence");
                    if ui
                        .selectable_value(&mut self.tab, Tab::Files, "Files")
                        .clicked()
                    {
                        self.files = files_in(&self.dir, "toml");
                    }
                    let dirty = self.edits.trimmed() != self.saved.trimmed();
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
                        Tab::Sequence => self.sequence_ui(ui, io),
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
                let away = if kind == LightType::Spot { 25.0 } else { 6.0 };
                let Some(rig) = Self::rig_mut(&mut self.edits, self.scope, io.scene) else {
                    self.message =
                        "this scene uses the default lights: give it its own first".into();
                    return;
                };
                let count = rig.lights.iter().filter(|l| l.kind == kind).count();
                let light = SceneLight {
                    name: format!("{} {}", kind.name(), count + 1),
                    position: [
                        p.position[0] + n[0] * away,
                        p.position[1] + n[1] * away,
                        p.position[2] + n[2] * away,
                    ],
                    direction: n.map(|v| -v),
                    ..SceneLight::new(kind)
                };
                rig.lights.push(light);
                self.light = Some(rig.lights.len() - 1);
                self.message = "light added".into();
            }
            Mode::Focus => {
                self.mode = Mode::Select;
                let (Some(k), Some(i), Some((eye, _))) = (self.sequence, self.key, io.camera)
                else {
                    return;
                };
                if p.empty {
                    self.message = "nothing there to focus on".into();
                    return;
                }
                let d = (0..3)
                    .map(|j| (p.position[j] - eye[j]).powi(2))
                    .sum::<f32>()
                    .sqrt();
                if let Some(key) = self.seq.list.get_mut(k).and_then(|s| s.keys.get_mut(i)) {
                    key.focus = d;
                    self.message = format!("focus {d:.1} units away");
                }
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

    /// Draws and works the lights' handles; true when they take the
    /// pointer.
    fn handles(&mut self, ctx: &egui::Context, io: &Io) -> bool {
        let (Some((view_proj, eye)), Some((_, look_at))) = (io.view, io.camera) else {
            return false;
        };
        let view = crate::gizmo::View {
            view_proj,
            eye,
            look_at,
            screen: ctx.content_rect(),
        };
        let scene = io.scene.to_owned();
        let editable = self.scope == Scope::Global
            || self
                .edits
                .scenes
                .get(&scene)
                .is_some_and(|s| s.rig.is_some());
        let lights = self.edits.rig_for(&scene).lights.clone();
        let mut edit = self
            .light
            .filter(|_| editable)
            .and_then(|k| lights.get(k).cloned());
        let before = edit.clone();
        let (clicked, taken) = self.gizmo.ui(ctx, &view, &lights, self.light, &mut edit);
        if let Some(k) = clicked {
            self.light = Some(k);
        }
        if edit != before
            && let (Some(k), Some(l)) = (self.light, edit)
            && let Some(rig) = Self::rig_mut(&mut self.edits, self.scope, &scene)
            && let Some(slot) = rig.lights.get_mut(k)
        {
            *slot = l;
        }
        taken
    }

    /// The rig changes go to: the default (made from the built-in one when
    /// missing), or the scene's own (none while it uses the default).
    fn rig_mut<'a>(edits: &'a mut Edits, scope: Scope, scene: &str) -> Option<&'a mut Rig> {
        match scope {
            Scope::Global => Some(edits.global.rig.get_or_insert_with(Rig::default)),
            Scope::Scene => edits.scenes.get_mut(scene).and_then(|s| s.rig.as_mut()),
        }
    }

    fn profile_ui(&mut self, ui: &mut egui::Ui, io: &Io) {
        let scope = self.scope;
        let target = Self::target(&mut self.edits, scope, io.scene);
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
    }

    #[allow(clippy::too_many_lines)] // the rig's header, list and the glows
    fn light_ui(&mut self, ui: &mut egui::Ui, io: &mut Io) {
        self.profile_ui(ui, io);
        let scope = self.scope;
        let scene = io.scene.to_owned();
        ui.separator();
        // Which lights: the default rig, or the scene's own.
        let own = self
            .edits
            .scenes
            .get(&scene)
            .is_some_and(|s| s.rig.is_some());
        match scope {
            Scope::Global => {
                ui.label("The default lights: every scene without lights of its own.");
            }
            Scope::Scene if own => {
                ui.horizontal(|ui| {
                    ui.label("This scene has lights of its own.");
                    if ui
                        .button("Back to the default")
                        .on_hover_text("Its own lights are deleted")
                        .clicked()
                    {
                        self.edits.scene_mut(&scene).rig = None;
                        self.light = None;
                    }
                });
            }
            Scope::Scene => {
                ui.horizontal(|ui| {
                    ui.label("This scene uses the default lights.");
                    if ui
                        .button("Give it its own")
                        .on_hover_text("A copy of the default lights to change for this scene")
                        .clicked()
                    {
                        let copy = self.edits.default_rig().clone();
                        self.edits.scene_mut(&scene).rig = Some(copy);
                    }
                });
            }
        }
        let editable = scope == Scope::Global || own;
        let mut shown = self.edits.rig_for(&scene).clone();
        let before = shown.clone();
        let mut place = None;
        ui.add_enabled_ui(editable, |ui| {
            ui.horizontal(|ui| {
                ui.label("Exposure");
                ui.add(
                    egui::DragValue::new(&mut shown.exposure)
                        .speed(0.05)
                        .range(-6.0..=20.0)
                        .suffix(" EV100"),
                )
                .on_hover_text(
                    "Higher is darker: bright day 14–15, overcast 12, interiors 7–9, night 2–4",
                );
            });
            ui.horizontal(|ui| {
                ui.label("Handles");
                use crate::gizmo::Mode as G;
                ui.selectable_value(&mut self.gizmo.mode, G::Move, "Move")
                    .on_hover_text("Arrows along the world axes (point and spot lights)");
                ui.selectable_value(&mut self.gizmo.mode, G::Rotate, "Rotate")
                    .on_hover_text("Rings about the world axes (directional and spot lights)");
                ui.selectable_value(&mut self.gizmo.mode, G::Scale, "Scale")
                    .on_hover_text("The circle of its reach (attenuation radius)");
            });
            ui.horizontal(|ui| {
                ui.strong("Lights");
                ui.menu_button("+ Add", |ui| {
                    for kind in LightType::ALL {
                        let placed = matches!(kind, LightType::Point | LightType::Spot);
                        let text = if placed {
                            format!("{} (click where it goes)", kind.name())
                        } else {
                            kind.name().to_owned()
                        };
                        if ui.button(text).clicked() {
                            if placed {
                                place = Some(kind);
                            } else {
                                shown.lights.push(SceneLight::new(kind));
                                self.light = Some(shown.lights.len() - 1);
                            }
                            ui.close();
                        }
                    }
                });
            });
            let mut remove = None;
            let mut copy = None;
            for (k, l) in shown.lights.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut l.enabled, "");
                    let mut text = format!("{}  ·  {}", l.name, l.kind.name());
                    if l.game != GameLight::None {
                        text.push_str("  · game");
                    }
                    if l.mobility == Mobility::Static {
                        text.push_str("  · static");
                    }
                    if ui.selectable_label(self.light == Some(k), text).clicked() {
                        self.light = (self.light != Some(k)).then_some(k);
                    }
                });
                if self.light != Some(k) {
                    continue;
                }
                ui.indent(k, |ui| {
                    details(ui, l, io);
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
                let mut l = shown.lights[k].clone();
                l.name.push_str(" copy");
                l.position[0] += 5.0;
                shown.lights.push(l);
                self.light = Some(shown.lights.len() - 1);
            }
            if let Some(k) = remove {
                shown.lights.remove(k);
                self.light = None;
            }
            if shown.lights.is_empty() {
                ui.label("No lights: the scene is dark.");
            }
        });
        if let Some(kind) = place {
            self.mode = Mode::Place(kind);
            self.message = if kind == LightType::Spot {
                "click what the spot light shines on".into()
            } else {
                "click in the scene where the light goes".into()
            };
        }
        if editable
            && shown != before
            && let Some(rig) = Self::rig_mut(&mut self.edits, scope, &scene)
        {
            *rig = shown;
        }
        ui.separator();
        self.glow_ui(ui, io);
    }

    fn glow_ui(&mut self, ui: &mut egui::Ui, io: &Io) {
        let scope = self.scope;
        let scene = io.scene.to_owned();
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
                            .on_hover_text("A light of your own in its place (in this scene's or the default lights)")
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
                    match Self::rig_mut(&mut self.edits, scope, &scene) {
                        Some(rig) => {
                            rig.lights.push(SceneLight {
                                name: g.name.clone(),
                                position: g.position,
                                color: g.color,
                                intensity: rig::candela_for(
                                    g.intensity * b,
                                    crate::enhanced::lights::REFERENCE,
                                ),
                                attenuation_radius: g.range,
                                cast_shadows: g.shadows,
                                ..SceneLight::new(LightType::Point)
                            });
                            self.light = Some(rig.lights.len() - 1);
                        }
                        None => {
                            self.message =
                                "this scene uses the default lights: give it its own first".into();
                        }
                    }
                }
            }
        });
    }

    #[allow(clippy::too_many_lines)] // recording, the list, the timeline, the keys
    fn sequence_ui(&mut self, ui: &mut egui::Ui, io: &mut Io) {
        let scene = io.scene.to_owned();
        ui.horizontal(|ui| {
            if let Some((_, frames)) = &self.seq.recording {
                if ui.button(format!("Stop ({} frames)", frames.len())).clicked()
                    && let Some(k) = self.seq.stop()
                {
                    self.sequence = Some(k);
                    self.cursor = 0;
                }
            } else if ui
                .add_enabled(!scene.is_empty(), egui::Button::new(egui::RichText::new("Record").color(egui::Color32::from_rgb(230, 80, 80))))
                .on_hover_text("Records the frames from now (a cutscene as it plays), until Stop or a new scene")
                .clicked()
            {
                self.seq.record(&scene);
            }
            let text = if self.paused { "▶ Resume game" } else { "⏸ Pause game" };
            if ui
                .button(text)
                .on_hover_text("The game stops; the overlay stays (hiding it resumes)")
                .clicked()
            {
                self.paused = !self.paused;
                self.playing = false;
            }
            if ui
                .add_enabled(self.seq.dirty(), egui::Button::new("Save"))
                .on_hover_text("Writes sequences.toml (the keyframes and camera paths)")
                .clicked()
            {
                self.seq.save();
            }
        });
        if !self.seq.message.is_empty() {
            ui.label(&self.seq.message);
        }
        ui.separator();
        ui.strong("Sequences in this scene");
        let mut any = false;
        for (k, s) in self.seq.list.iter().enumerate() {
            if s.scene != scene {
                continue;
            }
            any = true;
            let take = self.seq.take(k).is_some();
            let text = format!(
                "{}  ·  {} frames, {} keys{}",
                s.name,
                s.cameras.len(),
                s.keys.len(),
                if take { "  · recorded now" } else { "" }
            );
            if ui
                .selectable_label(self.sequence == Some(k), text)
                .clicked()
            {
                self.sequence = Some(k);
                self.cursor = 0;
                self.key = None;
            }
        }
        if !any {
            ui.label("None yet: Record while a cutscene plays.");
        }
        let Some(k) = self
            .sequence
            .filter(|&k| self.seq.list.get(k).is_some_and(|s| s.scene == scene))
        else {
            return;
        };
        ui.separator();
        let frames = self.seq.list[k].cameras.len().max(1);
        let take = self.seq.take(k).map(<[_]>::len);
        let mut delete = false;
        ui.horizontal(|ui| {
            ui.label("Name");
            ui.text_edit_singleline(&mut self.seq.list[k].name);
            delete = ui.button("Delete").clicked();
        });
        if delete {
            self.seq.delete(k);
            self.sequence = None;
            return;
        }
        // The timeline.
        if take.is_some() {
            ui.checkbox(
                &mut self.preview,
                "Show the recording while the game is paused",
            );
        } else {
            ui.label("Recorded in an earlier session: record it again to see it (the keys stay).");
        }
        ui.horizontal(|ui| {
            if ui.button("|◀").clicked() {
                self.cursor = 0;
            }
            if ui.button("◀").clicked() {
                self.cursor = self.cursor.saturating_sub(1);
            }
            let play = if self.playing { "❚❚" } else { "▶" };
            if ui
                .button(play)
                .on_hover_text("Plays the recording (the game paused)")
                .clicked()
            {
                self.playing = !self.playing;
                if self.playing {
                    self.paused = true;
                }
            }
            if ui.button("▶").clicked() {
                self.cursor = (self.cursor + 1).min(frames - 1);
            }
            if ui.button("▶|").clicked() {
                self.cursor = frames - 1;
            }
        });
        if self.playing {
            self.cursor += 1;
            if self.cursor >= frames {
                self.cursor = frames - 1;
                self.playing = false;
            }
        }
        let marks: Vec<u32> = self.seq.list[k].keys.iter().map(|k| k.frame).collect();
        self.cursor = timeline(ui, self.cursor, frames, &marks);
        // The keys.
        ui.horizontal(|ui| {
            ui.strong("Depth of field keys");
            if ui.button("+ Key here").clicked() {
                let at = self.cursor as u32;
                let base = self.seq.list[k].dof_at(at as f32).unwrap_or_default();
                let keys = &mut self.seq.list[k].keys;
                keys.retain(|x| x.frame != at);
                keys.push(crate::sequence::Key { frame: at, ..base });
                keys.sort_by_key(|x| x.frame);
                self.key = keys.iter().position(|x| x.frame == at);
            }
        });
        if self.mode == Mode::Focus {
            ui.label("Click what to focus on in the picture.");
        }
        let mut remove = None;
        egui::Grid::new("keys")
            .num_columns(6)
            .striped(true)
            .show(ui, |ui| {
                ui.label("frame");
                ui.label("focus");
                ui.label("strength");
                ui.label("radius");
                ui.label("");
                ui.label("");
                ui.end_row();
                for (i, key) in self.seq.list[k].keys.iter_mut().enumerate() {
                    if ui
                        .selectable_label(self.key == Some(i), key.frame.to_string())
                        .clicked()
                    {
                        self.key = Some(i);
                        self.cursor = key.frame as usize;
                    }
                    ui.add(
                        egui::DragValue::new(&mut key.focus)
                            .speed(0.5)
                            .range(0.0..=10000.0),
                    )
                    .on_hover_text("World units from the camera (0: automatic)");
                    ui.add(
                        egui::DragValue::new(&mut key.strength)
                            .speed(0.01)
                            .range(0.0..=4.0),
                    );
                    ui.add(
                        egui::DragValue::new(&mut key.radius)
                            .speed(0.1)
                            .range(0.0..=64.0),
                    );
                    if ui
                        .button("pick")
                        .on_hover_text("Click what to focus on")
                        .clicked()
                    {
                        self.key = Some(i);
                        self.mode = Mode::Focus;
                    }
                    if ui.button("Delete").clicked() {
                        remove = Some(i);
                    }
                    ui.end_row();
                }
            });
        if let Some(i) = remove {
            self.seq.list[k].keys.remove(i);
            self.key = None;
        }
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

/// One row of the details grid.
fn prop(ui: &mut egui::Ui, label: &str, hint: &str, add: impl FnOnce(&mut egui::Ui)) {
    let r = ui.label(label);
    if !hint.is_empty() {
        r.on_hover_text(hint);
    }
    ui.horizontal(add);
    ui.end_row();
}

fn drag(
    ui: &mut egui::Ui,
    v: &mut f32,
    speed: f64,
    range: std::ops::RangeInclusive<f32>,
    suffix: &str,
) {
    ui.add(
        egui::DragValue::new(v)
            .speed(speed)
            .range(range)
            .suffix(suffix),
    );
}

/// A light's properties, as Unreal's details panel lays them out.
#[allow(clippy::too_many_lines)] // one row per property
fn details(ui: &mut egui::Ui, l: &mut SceneLight, io: &Io) {
    let kind = l.kind;
    let local = matches!(kind, LightType::Point | LightType::Spot);
    let aimed = matches!(kind, LightType::Directional | LightType::Spot);
    let followed = match (l.game, io.game) {
        (GameLight::Key, Some(g)) => Some(g.key),
        (GameLight::Fill, Some(g)) => Some(g.fill),
        (GameLight::Ambient, Some(g)) => Some(([0.0, -1.0, 0.0], g.ambient)),
        _ => None,
    };
    egui::Grid::new("light details")
        .num_columns(2)
        .spacing([12.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            prop(ui, "Name", "", |ui| {
                ui.add(egui::TextEdit::singleline(&mut l.name).desired_width(170.0));
            });
            prop(
                ui,
                "Mobility",
                "Static: its shadows are made once and kept until it changes (cheap). Dynamic: worked out every frame.",
                |ui| {
                    ui.selectable_value(&mut l.mobility, Mobility::Static, "Static");
                    ui.selectable_value(&mut l.mobility, Mobility::Dynamic, "Dynamic");
                },
            );
            if l.game != GameLight::None {
                let what = match l.game {
                    GameLight::Key => "key light",
                    GameLight::Fill => "fill light",
                    _ => "ambient light",
                };
                prop(
                    ui,
                    "Follows the game",
                    "The map's own light for characters; what is ticked comes from the game every frame",
                    |ui| {
                        ui.label(what);
                        if aimed {
                            ui.checkbox(&mut l.follow_direction, "direction");
                        }
                        ui.checkbox(&mut l.follow_color, "colour");
                    },
                );
                if let Some((_, [r, g, b])) = followed {
                    prop(ui, "Game's colour", "Linear, with its brightness", |ui| {
                        ui.label(format!("{r:.2}, {g:.2}, {b:.2}"));
                    });
                }
            }
            if local {
                prop(ui, "Location", "World units (a character is about 20 tall)", |ui| {
                    for v in &mut l.position {
                        ui.add(egui::DragValue::new(v).speed(0.5));
                    }
                });
                if let Some((_, t)) = io.camera {
                    prop(ui, "", "", |ui| {
                        if ui.button("To what the camera looks at").clicked() {
                            l.position = [t[0], t[1] + 8.0, t[2]];
                        }
                    });
                }
            }
            if aimed {
                let fixed = followed.is_some() && l.follow_direction;
                prop(ui, "Rotation", "Pitch (down is negative) and yaw, degrees", |ui| {
                    ui.add_enabled_ui(!fixed, |ui| {
                        let (mut yaw, mut pitch) = angles(l.direction);
                        let before = (yaw, pitch);
                        ui.label("pitch");
                        ui.add(egui::DragValue::new(&mut pitch).speed(0.5).range(-90.0..=90.0).suffix("°"));
                        ui.label("yaw");
                        ui.add(egui::DragValue::new(&mut yaw).speed(0.5).range(-180.0..=180.0).suffix("°"));
                        if (yaw, pitch) != before {
                            l.direction = direction(yaw, pitch);
                        }
                    });
                });
            }
            let unit = format!(" {}", kind.unit());
            let hint = match kind {
                LightType::Original => "A multiple of the game's painted light",
                LightType::Sky => "Luminance of the sky, cd/m²",
                LightType::Directional if followed.is_some() && l.follow_color => {
                    "Illuminance at the game's brightness 1 (the game's light scales it)"
                }
                LightType::Directional => "Illuminance on a surface facing it (sun 30000–100000, overcast 1000–10000, moon 0.1–1)",
                _ => "Luminous intensity, candela (a 60 W bulb is about 70)",
            };
            prop(ui, "Intensity", hint, |ui| {
                let speed = f64::from(l.intensity.abs() * 0.01 + 0.01);
                drag(ui, &mut l.intensity, speed, 0.0..=1.0e7, &unit);
            });
            let tinted = followed.is_some() && l.follow_color;
            prop(ui, if kind == LightType::Sky { "Upper Color" } else { "Light Color" }, "", |ui| {
                ui.add_enabled_ui(!tinted && !l.use_temperature, |ui| {
                    ui.color_edit_button_rgb(&mut l.color);
                });
            });
            if kind == LightType::Sky {
                prop(ui, "Lower Color", "Light from below (the ground)", |ui| {
                    ui.color_edit_button_rgb(&mut l.lower_color);
                });
            }
            if kind != LightType::Original {
                prop(ui, "Use Temperature", "", |ui| {
                    ui.add_enabled_ui(!tinted, |ui| {
                        ui.checkbox(&mut l.use_temperature, "");
                        ui.add_enabled_ui(l.use_temperature, |ui| {
                            drag(ui, &mut l.temperature, 10.0, 1000.0..=40000.0, " K");
                        });
                    });
                });
            }
            match kind {
                LightType::Directional => prop(
                    ui,
                    "Source Angle",
                    "How large the light looks, degrees (the sun 0.5): softer shadow edges",
                    |ui| drag(ui, &mut l.source, 0.01, 0.0..=10.0, "°"),
                ),
                LightType::Point | LightType::Spot => {
                    prop(ui, "Source Radius", "World units", |ui| {
                        drag(ui, &mut l.source, 0.05, 0.0..=50.0, "");
                    });
                    prop(ui, "Attenuation Radius", "Where its light ends, world units", |ui| {
                        drag(ui, &mut l.attenuation_radius, 0.5, 1.0..=5000.0, "");
                    });
                }
                _ => {}
            }
            if kind == LightType::Spot {
                prop(ui, "Inner Cone Angle", "Full light inside, degrees", |ui| {
                    drag(ui, &mut l.inner_cone, 0.2, 0.0..=89.0, "°");
                });
                prop(ui, "Outer Cone Angle", "No light outside, degrees", |ui| {
                    drag(ui, &mut l.outer_cone, 0.2, 1.0..=89.0, "°");
                });
                l.inner_cone = l.inner_cone.min(l.outer_cone);
            }
            if matches!(kind, LightType::Directional | LightType::Point | LightType::Spot) {
                let hint = if kind == LightType::Directional {
                    "The first directional light with shadows is the key light: shadow cascades, ray-traced shadows, light shafts"
                } else {
                    "Ray-traced from the scenery, where the GPU traces rays"
                };
                prop(ui, "Cast Shadows", hint, |ui| {
                    ui.checkbox(&mut l.cast_shadows, "");
                });
            }
            prop(ui, "Affects Scenery", "Times its light on scenery (0: none)", |ui| {
                drag(ui, &mut l.scenery, 0.01, 0.0..=4.0, " ×");
            });
            prop(ui, "Affects Characters", "Times its light on characters (0: none)", |ui| {
                drag(ui, &mut l.characters, 0.01, 0.0..=4.0, " ×");
            });
            if kind != LightType::Original && kind != LightType::Sky {
                prop(ui, "Specular Scale", "Its highlights", |ui| {
                    drag(ui, &mut l.specular, 0.01, 0.0..=4.0, "");
                });
            }
            if kind == LightType::Directional {
                prop(ui, "Volumetric Scattering", "Light shafts in the air (the key light)", |ui| {
                    drag(ui, &mut l.volumetric, 0.01, 0.0..=10.0, "");
                });
            }
            if kind == LightType::Original {
                prop(
                    ui,
                    "Indirect Intensity",
                    "Its light bounced from the surroundings (light probes): on characters, and on scenery as far as Affects Scenery is below 1",
                    |ui| drag(ui, &mut l.indirect, 0.01, 0.0..=4.0, ""),
                );
                prop(ui, "Key Shadows", "How much the key light's shadows darken it on scenery", |ui| {
                    drag(ui, &mut l.key_shadows, 0.01, 0.0..=1.0, "");
                });
                prop(ui, "Ambient Occlusion", "Screen-space occlusion on it", |ui| {
                    drag(ui, &mut l.occlusion, 0.01, 0.0..=1.0, "");
                });
                prop(ui, "Key Highlights", "The key light's highlights on scenery where it is bright", |ui| {
                    drag(ui, &mut l.highlights, 0.01, 0.0..=4.0, "");
                });
            }
        });
}

/// The sequence's timeline: frames along a track, the keys as ticks and
/// the cursor as a handle at the same places (click or drag to move it).
/// Returns the cursor.
fn timeline(ui: &mut egui::Ui, cursor: usize, frames: usize, keys: &[u32]) -> usize {
    let last = frames.saturating_sub(1).max(1) as f32;
    let width = ui.available_width().max(120.0);
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, 34.0), egui::Sense::click_and_drag());
    let track = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 8.0, rect.top() + 10.0),
        egui::pos2(rect.right() - 8.0, rect.top() + 16.0),
    );
    let x_of = |f: f32| track.left() + track.width() * (f / last).clamp(0.0, 1.0);
    let mut cursor = cursor.min(frames.saturating_sub(1));
    if let Some(p) = response.interact_pointer_pos()
        && (response.dragged() || response.clicked())
    {
        cursor = (((p.x - track.left()) / track.width()).clamp(0.0, 1.0) * last).round() as usize;
    }
    let painter = ui.painter();
    let visuals = ui.visuals();
    painter.rect_filled(track, 3.0, visuals.widgets.inactive.bg_fill);
    for &k in keys {
        let x = x_of(k as f32);
        painter.line_segment(
            [
                egui::pos2(x, track.top() - 4.0),
                egui::pos2(x, track.bottom() + 8.0),
            ],
            egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 200, 60)),
        );
    }
    let x = x_of(cursor as f32);
    painter.circle_filled(
        egui::pos2(x, track.center().y),
        6.0,
        visuals.selection.bg_fill,
    );
    painter.text(
        egui::pos2(rect.left() + 8.0, rect.bottom()),
        egui::Align2::LEFT_BOTTOM,
        format!("frame {cursor} of {}", frames.saturating_sub(1)),
        egui::FontId::proportional(12.0),
        visuals.text_color(),
    );
    cursor
}
