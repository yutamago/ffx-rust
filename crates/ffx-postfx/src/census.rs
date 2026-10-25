//! The census of one captured frame's draws outside the scene targets
//! (`docs/formats/postfx.md`).
//!
//! Every draw is put in a class: the scene (two colour targets and a
//! depth buffer, as `ffx-drawdiff` tells them), one of the post-process
//! passes, the effects (`vfx*` shaders, `PhyreScene_DrawMostVFX`) or the
//! UI (`menu*` shaders, `PhyreScene_DrawNormalUI`). The post-process
//! draws then have every used constant checked against where FFX.exe
//! takes it from.

use std::fmt::Write as _;

use ffx_drawdiff::capture::FrameItems;
use ffx_drawdiff::{Item, ItemKind, Value};

use crate::camera::Camera;
use crate::shaders::ShaderNames;

/// A post-process pass, in `PhyreScene_render`'s order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pass {
    /// `PostProcessManager_prepareGlowMap` (`0x00694c60`): the colour
    /// through the glow thresholds into the glow map (`+0xa4`).
    GlowMap,
    /// `Phyre_PPostProcessing_PLegacyGlowGPUBase_render` (`0x00a1de20`, on the
    /// post-process manager's `+0x1100`): a Gaussian blur pass of level
    /// `level`, going down (`up` false) or back up; `vertical` the second
    /// pass of the pair (on the way up: blurred and added).
    GlowBlur {
        level: u32,
        up: bool,
        vertical: bool,
    },
    /// `Distortion_renderDistortion` (`0x006c2dc0`): the colour copied into the
    /// distortion buffer with alpha (`PostProcessManager_copyDepthBuffer` `0x00692e90` with a `ColorScale`).
    DistortionCopy,
    /// `Distortion_renderDistortion`: the distortion grid drawn back over the
    /// cleared colour.
    DistortionGrid,
    /// `PostProcessManager_renderBright` (`0x00695440`): the colour
    /// through the brightness setting into `+0xa4`.
    Bright,
    /// `renderBright`'s `copyBuffer` (`0x00692c40`) of `+0xa4` back into
    /// the cleared colour.
    BrightCopy,
    /// `PhyreScene_render`'s last `copyBuffer`: the colour to the back
    /// buffer.
    Composite,
}

impl Pass {
    #[must_use]
    pub fn name(self) -> String {
        match self {
            Self::GlowMap => "glow map".into(),
            Self::GlowBlur {
                level,
                up,
                vertical,
            } => format!(
                "glow {} level {level} {}",
                if up { "up" } else { "down" },
                match (up, vertical) {
                    (_, false) => "h",
                    (false, true) => "v",
                    (true, true) => "v+add",
                }
            ),
            Self::DistortionCopy => "distortion copy".into(),
            Self::DistortionGrid => "distortion grid".into(),
            Self::Bright => "brightness".into(),
            Self::BrightCopy => "brightness copy back".into(),
            Self::Composite => "composite".into(),
        }
    }

    /// The FFX.exe function that issues it.
    #[must_use]
    pub fn function(self) -> &'static str {
        match self {
            Self::GlowMap => "PostProcessManager_prepareGlowMap 0x00694c60",
            Self::GlowBlur { .. } => "Phyre_PPostProcessing_PLegacyGlowGPUBase_render 0x00a1de20",
            Self::DistortionCopy => {
                "Distortion_renderDistortion 0x006c2dc0 (PostProcessManager_copyDepthBuffer 0x00692e90 with ColorScale)"
            }
            Self::DistortionGrid => "Distortion_renderDistortion 0x006c2dc0",
            Self::Bright => "PostProcessManager_renderBright 0x00695440",
            Self::BrightCopy => {
                "PostProcessManager_renderBright 0x00695440 (copyBuffer 0x00692c40)"
            }
            Self::Composite => "PhyreScene_render 0x00668930 (copyBuffer 0x00692c40)",
        }
    }

    /// Place in `PhyreScene_render`'s order (effects and UI between).
    fn rank(self) -> u32 {
        match self {
            Self::GlowMap => 1,
            Self::GlowBlur { .. } => 2,
            Self::DistortionCopy => 4,
            Self::DistortionGrid => 5,
            Self::Bright => 6,
            Self::BrightCopy => 7,
            Self::Composite => 9,
        }
    }
}

/// What a draw is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    Scene,
    Post(Pass),
    /// `PhyreScene_DrawMostVFX` (the `pc-effects` census).
    Effects,
    /// `PhyreScene_DrawNormalUI` (the `pc-ui2d` census).
    Ui,
    Unclassified,
}

impl Class {
    #[must_use]
    pub fn name(self) -> String {
        match self {
            Self::Scene => "scene".into(),
            Self::Post(p) => format!("post: {}", p.name()),
            Self::Effects => "effects".into(),
            Self::Ui => "ui".into(),
            Self::Unclassified => "UNCLASSIFIED".into(),
        }
    }

    fn rank(self) -> u32 {
        match self {
            Self::Scene => 0,
            Self::Post(p) => p.rank(),
            Self::Effects => 3,
            Self::Ui => 8,
            Self::Unclassified => u32::MAX,
        }
    }
}

/// How a constant compares with its source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Bit for bit what the source gives.
    Match,
    /// Game state the census cannot read from the data; the value is in
    /// the range the code allows.
    GameState,
    /// Not what the source gives: (expected words).
    Mismatch(Vec<u32>),
    /// No source known.
    Unknown,
}

/// One used constant of a post-process draw.
#[derive(Debug, Clone)]
pub struct Constant {
    pub stage: &'static str,
    pub var: String,
    pub words: Vec<Option<u32>>,
    /// Where FFX.exe takes it from.
    pub source: String,
    pub verdict: Verdict,
}

/// One draw of the frame.
#[derive(Debug, Clone)]
pub struct DrawCensus {
    /// 1-based, as `frame.txt` numbers draws.
    pub number: usize,
    /// Index among the frame's items.
    pub item: usize,
    pub class: Class,
    pub vs_family: String,
    pub ps_family: String,
    /// `rtv0`, `ps.srv0` labels.
    pub target: String,
    pub input: String,
    pub blend: String,
    pub constants: Vec<Constant>,
}

/// A clear outside the scene targets, with the pass it belongs to.
#[derive(Debug, Clone)]
pub struct ClearCensus {
    pub item: usize,
    pub label: String,
    pub owner: String,
}

/// What the census needs besides the capture.
#[derive(Debug, Clone)]
pub struct Inputs<'a> {
    pub shaders: &'a ShaderNames,
    /// The map's cameras ([`crate::camera::map_cameras`]).
    pub cameras: &'a [Camera],
    /// `GameSetting.ini`'s `Brightness` (`g_Settings +0x24`).
    pub brightness: i32,
}

/// The whole frame.
#[derive(Debug, Clone)]
pub struct Census {
    pub draws: Vec<DrawCensus>,
    pub clears: Vec<ClearCensus>,
    /// The camera taken as the `GlobalCamera` (its `m_bgColor` is the
    /// scene's clear colour), and how many cameras qualified.
    pub camera: Option<Camera>,
    pub camera_candidates: usize,
    /// Draws out of `PhyreScene_render`'s order (draw numbers).
    pub out_of_order: Vec<usize>,
}

fn field<'a>(it: &'a Item, name: &str) -> Option<&'a Value> {
    it.fields.iter().find(|f| f.name == name).map(|f| &f.value)
}

fn label(it: &Item, name: &str) -> String {
    match field(it, name) {
        Some(Value::Key(_, l)) => l.clone(),
        Some(v) => v.to_string(),
        None => "-".into(),
    }
}

fn f32_field(it: &Item, name: &str) -> Option<f32> {
    match field(it, name)? {
        Value::F(b) => Some(f32::from_bits(*b)),
        _ => None,
    }
}

/// `WxH` of a target or texture label (`target9 640x360 f28 ...`).
fn label_size(l: &str) -> Option<(u32, u32)> {
    l.split_whitespace().find_map(|t| {
        let (w, h) = t.split_once('x')?;
        Some((w.parse().ok()?, h.parse().ok()?))
    })
}

fn has_var(it: &Item, name: &str) -> bool {
    it.constants
        .iter()
        .any(|k| k.vars.iter().any(|v| &*v.name == name))
}

fn blend_summary(it: &Item) -> String {
    match field(it, "blend.rt0.enable") {
        Some(Value::U(1)) => format!(
            "src {} dest {}",
            label(it, "blend.rt0.src"),
            label(it, "blend.rt0.dest")
        ),
        _ => "off".into(),
    }
}

/// Classes before the glow passes are numbered.
fn first_class(it: &Item, names: &ShaderNames) -> Class {
    if it.scene {
        return Class::Scene;
    }
    let ps = it.ps;
    if names.is(ps, "phyreglow") {
        if has_var(it, "FullGlowThreshold") {
            Class::Post(Pass::GlowMap)
        } else {
            Class::Post(Pass::GlowBlur {
                level: 0,
                up: false,
                vertical: false,
            })
        }
    } else if names.is(ps, "phyredistortionshader") {
        Class::Post(Pass::DistortionGrid)
    } else if names.is(ps, "brightnessshader") {
        Class::Post(Pass::Bright)
    } else if names.is(ps, "copybuffer") {
        if has_var(it, "ColorScale") {
            Class::Post(Pass::DistortionCopy)
        } else if label(it, "rtv0").contains("(undeclared)") {
            // The swap chain's back buffer is the one target the capture
            // never declares.
            Class::Post(Pass::Composite)
        } else {
            Class::Post(Pass::BrightCopy)
        }
    } else if names.is(ps, "vfx") || names.is(ps, "menu") {
        // Effects or UI: which one is told by the place in the frame
        // ([`split_effects_and_ui`]); the minimap draws with a vfx shader
        // and some vfx and menu shaders share their bytecode.
        Class::Effects
    } else {
        Class::Unclassified
    }
}

/// `PhyreScene_DrawMostVFX` runs between the glow and the post-effects
/// chain, `PhyreScene_DrawNormalUI` after it: an effects-or-UI draw after
/// the first pass of that chain (distortion, brightness) is UI.
fn split_effects_and_ui(classes: &mut [Class]) {
    let mut after_chain = false;
    for c in classes.iter_mut() {
        match c {
            Class::Post(p) if p.rank() > 3 && *p != Pass::Composite => after_chain = true,
            Class::Effects if after_chain => *c = Class::Ui,
            _ => {}
        }
    }
}

/// The scene's clear colour: the first clear of a scene target that is
/// not white (the normal target clears to 1, 1, 1, 1).
fn scene_clear(frame: &FrameItems) -> Option<[f32; 3]> {
    frame.items.iter().find_map(|it| {
        if it.kind != ItemKind::ClearTarget || !it.scene {
            return None;
        }
        let c = |n: &str| f32_field(it, &format!("clear.color.{n}"));
        let rgb = [c("r")?, c("g")?, c("b")?];
        (bits(&rgb) != bits(&[1.0; 3])).then_some(rgb)
    })
}

fn words_of(v: &ffx_drawdiff::cb::Var) -> Vec<Option<u32>> {
    v.words.iter().map(|(w, _)| *w).collect()
}

fn bits(v: &[f32]) -> Vec<u32> {
    v.iter().map(|f| f.to_bits()).collect()
}

fn verdict(words: &[Option<u32>], expected: &[u32]) -> Verdict {
    let got: Vec<u32> = words.iter().map(|w| w.unwrap_or(0xffff_ffff)).collect();
    if got == expected {
        Verdict::Match
    } else {
        Verdict::Mismatch(expected.to_vec())
    }
}

/// The glow's own parameter state through the frame (the `PLegacyGlow`
/// object's parameter buffer keeps what was last written, so a pass that
/// does not set a value sees the previous pass's).
#[derive(Default)]
struct GlowState {
    output_scale: Option<[f32; 4]>,
    /// Size of `+0x78[level]` (the down-h targets), by level.
    level_size: Vec<(u32, u32)>,
}

/// The x87 arithmetic of `0x00a1de20`, rounded to `f32` where it stores.
fn down_scale(level: u32, spread: f32) -> f32 {
    #[allow(clippy::cast_possible_truncation)] // the f32 store
    let s = (f64::from(level + 1) * f64::from(spread) + 1.0) as f32;
    s
}

fn up_scale(level: u32, spread: f32, up_gain: f32) -> f32 {
    if level == 0 {
        return 1.0;
    }
    #[allow(clippy::cast_possible_truncation)] // the f32 stores
    let base = if level < 3 {
        (1.0 / (f64::from(spread) * 2.0 + 1.0)) as f32
    } else {
        3.0
    };
    #[allow(clippy::cast_possible_truncation)]
    let s = (f64::from(up_gain) * f64::from(base)) as f32;
    s
}

/// `PLegacyGlow +0x5c`: 1.0 from its constructor (`0x009fc540`), never
/// written again.
const GLOW_UP_GAIN: f32 = 1.0;

#[allow(clippy::too_many_lines)] // one arm per constant
fn check_constants(
    pass: Pass,
    it: &Item,
    camera: Option<&Camera>,
    brightness: i32,
    glow: &mut GlowState,
) -> Vec<Constant> {
    let mut out = Vec::new();
    for k in &it.constants {
        for v in &k.vars {
            let words = words_of(v);
            let name = v.name.to_string();
            let (source, verdict) = match (pass, name.as_str()) {
                (_, "UvScaleBias") => (
                    "literal (1, 1, 0, 0) in the function; (2/3, 2/3, 0, 0) with copyBuffer's \
                     4:3 flag and post-process +0x123d9"
                        .to_owned(),
                    verdict(&words, &bits(&[1.0, 1.0, 0.0, 0.0])),
                ),
                (Pass::GlowMap, "FullGlowThreshold") => (
                    "scene context +0x80 = GlobalCamera m_glowUpperThreshold (+0x104), \
                     setupPostProcessParams 0x0066f140"
                        .to_owned(),
                    camera.map_or(Verdict::Unknown, |c| {
                        verdict(&words, &bits(&[c.glow_upper]))
                    }),
                ),
                (Pass::GlowMap, "NoGlowThreshold") => (
                    "scene context +0x84 = max(GlobalCamera m_glowLowerThreshold (+0x108), 0.35)"
                        .to_owned(),
                    camera.map_or(Verdict::Unknown, |c| {
                        verdict(&words, &bits(&[c.no_glow_threshold()]))
                    }),
                ),
                (Pass::GlowBlur { level, .. }, "GaussianBlurBufferSize") => {
                    let size = glow.level_size.get(level as usize).copied();
                    (
                        "derived: size of the level's first target (PLegacyGlow +0x78[level]), \
                         then 0, 0"
                            .to_owned(),
                        size.map_or(Verdict::Unknown, |(w, h)| {
                            #[allow(clippy::cast_precision_loss)] // sizes are small
                            let e = [w as f32, h as f32, 0.0, 0.0];
                            verdict(&words, &bits(&e))
                        }),
                    )
                }
                (
                    Pass::GlowBlur {
                        level,
                        up,
                        vertical,
                    },
                    "GaussianOutputScale",
                ) => {
                    let spread = camera.map(|c| c.glow_spread);
                    let written = match (up, vertical) {
                        (false, false) => spread.map(|s| {
                            let d = down_scale(level, s);
                            [d, d, d, 1.0]
                        }),
                        (true, false) if level == 0 => Some([1.0; 4]),
                        (true, false) | (false, true) => None,
                        (true, true) => spread.map(|s| {
                            let u = up_scale(level, s, GLOW_UP_GAIN);
                            [u, u, u, 1.0]
                        }),
                    };
                    if let Some(w) = written {
                        glow.output_scale = Some(w);
                    }
                    let how = match (up, vertical, written.is_some()) {
                        (false, false, _) => {
                            "derived: (level + 1) * GlobalCamera m_glowSpread + 1 (x3), 1"
                        }
                        (true, false, true) => "literal (1, 1, 1, 1) at level 0",
                        (true, true, _) => {
                            "derived: level 0: 1; below 3: PLegacyGlow +0x5c (1.0) / \
                             (2 m_glowSpread + 1); else 3 (x3), 1"
                        }
                        _ => "not written: the previous pass's value stays in the buffer",
                    };
                    (
                        how.to_owned(),
                        glow.output_scale
                            .map_or(Verdict::Unknown, |e| verdict(&words, &bits(&e))),
                    )
                }
                (Pass::GlowBlur { .. }, "GlowGain") => (
                    "PLegacyGlow +0x54 = scene context +0x88 = GlobalCamera m_glowGain (+0x10c), \
                     PostProcessManager_render 0x00694f10"
                        .to_owned(),
                    camera.map_or(Verdict::Unknown, |c| verdict(&words, &bits(&[c.glow_gain]))),
                ),
                (Pass::DistortionCopy, "ColorScale") => {
                    let rgb_one = words.iter().take(3).all(|w| *w == Some(1.0f32.to_bits()));
                    let a_ok = words
                        .get(3)
                        .copied()
                        .flatten()
                        .is_some_and(|a| f32::from_bits(a) <= f32::from_bits(0x3f4c_cccd));
                    (
                        "game state: (1, 1, 1, min(Distortion +0x78, 0.8)), Distortion_renderDistortion"
                            .to_owned(),
                        if rgb_one && a_ok {
                            Verdict::GameState
                        } else {
                            Verdict::Mismatch(Vec::new())
                        },
                    )
                }
                (Pass::Bright, "g_brightPara") => {
                    #[allow(clippy::cast_possible_truncation)] // the f32 store
                    let e = ((f64::from(brightness) - 50.0) / 100.0) as f32;
                    (
                        "setting: (post-process +0x12328 - 50) / 100, +0x12328 = Brightness \
                         (g_Settings +0x24, PhyreScene_ApplySettings 0x0066f010)"
                            .to_owned(),
                        verdict(&words, &bits(&[e])),
                    )
                }
                _ => ("?".to_owned(), Verdict::Unknown),
            };
            out.push(Constant {
                stage: k.stage,
                var: name,
                words,
                source,
                verdict,
            });
        }
    }
    out
}

/// Numbers the glow blur passes: `4 * levels` draws after the glow map,
/// first down (h, v per level), then up from the last level.
fn number_glow(classes: &mut [Class]) {
    let idx: Vec<usize> = classes
        .iter()
        .enumerate()
        .filter(|(_, c)| matches!(c, Class::Post(Pass::GlowBlur { .. })))
        .map(|(i, _)| i)
        .collect();
    let n = u32::try_from(idx.len()).unwrap_or(0);
    let levels = n / 4;
    for (k, &i) in idx.iter().enumerate() {
        let k = u32::try_from(k).unwrap_or(0);
        let (level, up, vertical) = if k < 2 * levels {
            (k / 2, false, k % 2 == 1)
        } else {
            let j = k - 2 * levels;
            (levels - 1 - j / 2, true, j % 2 == 1)
        };
        classes[i] = Class::Post(Pass::GlowBlur {
            level,
            up,
            vertical,
        });
    }
}

/// The census of one frame.
#[must_use]
pub fn census(frame: &FrameItems, inputs: &Inputs<'_>) -> Census {
    let names = inputs.shaders;
    let clear = scene_clear(frame);
    let candidates: Vec<&Camera> = inputs
        .cameras
        .iter()
        .filter(|c| clear.is_some_and(|rgb| bits(&c.bg) == bits(&rgb)))
        .collect();
    let camera = candidates.first().map(|c| (*c).clone());
    let draw_items: Vec<(usize, &Item)> = frame
        .items
        .iter()
        .enumerate()
        .filter(|(_, it)| it.kind == ItemKind::Draw)
        .collect();
    let mut classes: Vec<Class> = draw_items
        .iter()
        .map(|(_, it)| first_class(it, names))
        .collect();
    number_glow(&mut classes);
    split_effects_and_ui(&mut classes);
    let mut glow = GlowState::default();
    // The levels' first targets, from the down-h passes.
    for ((_, it), c) in draw_items.iter().zip(&classes) {
        if let Class::Post(Pass::GlowBlur {
            up: false,
            vertical: false,
            ..
        }) = c
        {
            let size = label_size(&label(it, "rtv0")).unwrap_or((0, 0));
            glow.level_size.push(size);
        }
    }
    let mut draws = Vec::new();
    let mut out_of_order = Vec::new();
    let mut last_rank = 0;
    for (n, ((item, it), class)) in draw_items.iter().zip(&classes).enumerate() {
        let constants = match class {
            Class::Post(p) => {
                check_constants(*p, it, camera.as_ref(), inputs.brightness, &mut glow)
            }
            _ => Vec::new(),
        };
        let rank = class.rank();
        if rank < last_rank {
            out_of_order.push(n + 1);
        }
        last_rank = last_rank.max(rank);
        draws.push(DrawCensus {
            number: n + 1,
            item: *item,
            class: *class,
            vs_family: names.family(it.vs),
            ps_family: names.family(it.ps),
            target: label(it, "rtv0"),
            input: label(it, "ps.srv0"),
            blend: blend_summary(it),
            constants,
        });
    }
    let clears = clears(frame, &draws);
    Census {
        draws,
        clears,
        camera,
        camera_candidates: candidates.len(),
        out_of_order,
    }
}

/// The non-scene clears, each given to the pass of the next draw into the
/// same target, else to `PhyreScene_render` itself.
fn clears(frame: &FrameItems, draws: &[DrawCensus]) -> Vec<ClearCensus> {
    let mut out = Vec::new();
    let last_scene = draws
        .iter()
        .filter(|d| d.class == Class::Scene)
        .map(|d| d.item)
        .max()
        .unwrap_or(0);
    for (i, it) in frame.items.iter().enumerate() {
        if it.kind == ItemKind::Draw || (it.scene && i < last_scene) {
            continue;
        }
        let target = label(it, "target");
        let next = draws.iter().find(|d| d.item > i);
        let owner = match next {
            Some(d) if it.kind == ItemKind::ClearTarget && d.target == target => d.class.name(),
            Some(d) if it.kind == ItemKind::ClearDepth && d.class != Class::Scene => {
                format!("{} (its depth target)", d.class.name())
            }
            Some(d) if d.class == Class::Scene => {
                "PhyreScene_render: the back buffer, (0, 0, 0, 1)".into()
            }
            Some(_) => "PhyreScene_render: post-process +0x9c (HDAO / postOpaque target)".into(),
            None => "after the last draw".into(),
        };
        out.push(ClearCensus {
            item: i,
            label: it.label.clone(),
            owner,
        });
    }
    out
}

impl Census {
    /// Draws by class name, in first-seen order.
    #[must_use]
    pub fn totals(&self) -> Vec<(String, usize)> {
        let mut out: Vec<(String, usize)> = Vec::new();
        for d in &self.draws {
            let key = match d.class {
                Class::Post(_) => "post".to_owned(),
                c => c.name(),
            };
            match out.iter_mut().find(|(k, _)| *k == key) {
                Some((_, n)) => *n += 1,
                None => out.push((key, 1)),
            }
        }
        out
    }

    /// Post-process constants that do not follow their source, or have
    /// none.
    #[must_use]
    pub fn bad_constants(&self) -> Vec<String> {
        let mut out = Vec::new();
        for d in &self.draws {
            for c in &d.constants {
                if matches!(c.verdict, Verdict::Mismatch(_) | Verdict::Unknown) {
                    out.push(format!(
                        "draw {} {} {}.{}: {:?} ({})",
                        d.number,
                        d.class.name(),
                        c.stage,
                        c.var,
                        c.verdict,
                        c.source
                    ));
                }
            }
        }
        out
    }

    /// A text report: one line per non-scene draw and clear, then the
    /// constants of the post-process draws.
    #[must_use]
    pub fn report(&self) -> String {
        let mut s = String::new();
        let _ = writeln!(s, "== draws outside the scene ==");
        for d in self.draws.iter().filter(|d| d.class != Class::Scene) {
            let _ = writeln!(
                s,
                "{:3} item {:3} {:<28} ps {:<32} {} <- {} blend {}",
                d.number,
                d.item,
                d.class.name(),
                d.ps_family,
                short(&d.target),
                short(&d.input),
                d.blend
            );
        }
        let _ = writeln!(s, "\n== clears outside the scene ==");
        for c in &self.clears {
            let _ = writeln!(s, "item {:3} {}: {}", c.item, short(&c.label), c.owner);
        }
        let _ = writeln!(s, "\n== post-process constants ==");
        for d in &self.draws {
            if let Class::Post(p) = d.class {
                let _ = writeln!(s, "{} {} ({})", d.number, p.name(), p.function());
                for c in &d.constants {
                    let vals: Vec<String> = c
                        .words
                        .iter()
                        .map(|w| w.map_or("-".into(), |b| format!("{}", f32::from_bits(b))))
                        .collect();
                    let _ = writeln!(
                        s,
                        "    {}.{} = [{}] {:?}: {}",
                        c.stage,
                        c.var,
                        vals.join(", "),
                        c.verdict,
                        c.source
                    );
                }
            }
        }
        let _ = writeln!(s, "\n== totals ==");
        for (k, n) in self.totals() {
            let _ = writeln!(s, "{k}\t{n}");
        }
        match &self.camera {
            Some(c) => {
                let _ = writeln!(
                    s,
                    "GlobalCamera: {} #{} of {} ({} candidates by m_bgColor): glow {} {} {} {}, dof {:?}",
                    c.class,
                    c.index,
                    c.path,
                    self.camera_candidates,
                    c.glow_upper,
                    c.glow_lower,
                    c.glow_gain,
                    c.glow_spread,
                    c.dof
                );
            }
            None => {
                let _ = writeln!(s, "GlobalCamera: none found");
            }
        }
        let _ = writeln!(s, "out of order: {:?}", self.out_of_order);
        s
    }
}

/// A label without its view description.
fn short(l: &str) -> &str {
    l.split(" view ").next().unwrap_or(l)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glow_scales_follow_the_x87_arithmetic() {
        let spread = f32::from_bits(0x3f26_6666); // 0.65
        assert_eq!(down_scale(0, spread).to_bits(), 0x3fd3_3333); // 1.65
        assert_eq!(down_scale(1, spread), 2.3);
        assert_eq!(down_scale(2, spread).to_bits(), 2.949_999_8f32.to_bits());
        assert_eq!(up_scale(0, spread, 1.0), 1.0);
        assert_eq!(
            up_scale(1, spread, 1.0).to_bits(),
            0.434_782_62f32.to_bits()
        );
        assert_eq!(up_scale(3, spread, 1.0), 3.0);
    }

    #[test]
    fn glow_passes_are_numbered_down_then_up() {
        let blur = Class::Post(Pass::GlowBlur {
            level: 0,
            up: false,
            vertical: false,
        });
        let mut c = vec![Class::Post(Pass::GlowMap)];
        c.extend(std::iter::repeat_n(blur, 12));
        number_glow(&mut c);
        let names: Vec<String> = c.iter().map(|c| c.name()).collect();
        assert_eq!(names[1], "post: glow down level 0 h");
        assert_eq!(names[6], "post: glow down level 2 v");
        assert_eq!(names[7], "post: glow up level 2 h");
        assert_eq!(names[12], "post: glow up level 0 v+add");
    }

    #[test]
    fn sizes_come_from_labels() {
        assert_eq!(
            label_size("target9 640x360 f28 view [28, 4, 0, 0, 0, 0]"),
            Some((640, 360))
        );
        assert_eq!(label_size("target0 (undeclared) view [28]"), None);
    }
}
