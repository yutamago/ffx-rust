//! One game Flash screen played by Ruffle, headless (wgpu, offscreen):
//! what Iggy's player did for `FFX.exe`, mapped onto Ruffle's API
//! (`docs/formats/pc-ui.md`, "Iggy calls" and "Ruffle").
//!
//! - `IggyPlayerCallFunctionRS(name, args)` calls an AS3 function the movie
//!   registered with `ExternalInterface.addCallback`: Ruffle's
//!   `Player::call_internal_interface`, the arguments fitted to the
//!   function's count first as Iggy does ([`crate::abc::Arity::fit`]: Ruffle
//!   throws on a missing one, and the game calls `addChangeParaPageItem`
//!   with 4 of its 5).
//! - The AS3 side's `ExternalInterface.call(name, args)` reaches the game's
//!   callback (`IggySetAS3ExternalFunctionCallbackUTF8`): here an
//!   [`ExternalInterfaceProvider`] that answers with what the game answered
//!   in the recording and keeps the calls for comparison.
//! - `IggyPlayerTickRS` runs one Flash frame: `Player::tick` by one frame
//!   period (timers advance by the same time).
//! - Fonts: the movies name `$NormalFont` / `$TitleFont`; Iggy resolves
//!   them to the one font the game loads (`TBUDGothic Std B`, `box.swf`),
//!   so that font is registered as a device font under both names.
//! - Iggy draws the movie with a transparent background over the scene;
//!   Ruffle's stage is put in transparent window mode.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use anyhow::{Result, anyhow};
use ruffle_core::backend::ui::FontDefinition;
use ruffle_core::context::UpdateContext;
use ruffle_core::external::{ExternalInterfaceProvider, Value};
use ruffle_core::swf;
use ruffle_core::tag_utils::SwfMovieData;
use ruffle_core::{FloatDuration, Player, PlayerBuilder, ViewportDimensions};
use ruffle_render_wgpu::backend::{
    WgpuRenderBackend, create_wgpu_instance, request_adapter_and_device,
};
use ruffle_render_wgpu::descriptors::Descriptors;
use ruffle_render_wgpu::target::TextureTarget;
use ruffle_render_wgpu::wgpu;

use crate::abc::Arity;
use crate::log::Arg;

/// Ruffle's mouse buttons ([`Stage::mouse_button`]).
pub use ruffle_core::events::MouseButton;
/// Ruffle's external value, for users of [`CallOut`] (`ffx-host`).
pub use ruffle_core::external::Value as RuffleValue;

/// The GPU device every stage shares.
pub struct Gpu {
    descriptors: Arc<Descriptors>,
}

impl Gpu {
    /// Any adapter wgpu finds (headless).
    ///
    /// # Errors
    /// No adapter.
    pub fn new() -> Result<Self> {
        let backends = wgpu::Backends::all();
        let instance = create_wgpu_instance(backends, wgpu::BackendOptions::default(), None);
        let (adapter, device, queue) = futures::executor::block_on(request_adapter_and_device(
            backends,
            &instance,
            None,
            wgpu::PowerPreference::HighPerformance,
        ))
        .map_err(|e| anyhow!("{e}"))?;
        let info = adapter.get_info();
        eprintln!(
            "ffx-swf: wgpu {:?} on {} ({:?})",
            info.backend, info.name, info.device_type
        );
        Ok(Self {
            descriptors: Arc::new(Descriptors::new(instance, adapter, device, queue)),
        })
    }
}

/// An `ExternalInterface.call` the movie made.
#[derive(Debug, Clone, PartialEq)]
pub struct CallOut {
    pub name: String,
    pub args: Vec<Value>,
}

/// A live answer to the movie's external calls: called with the name and
/// the arguments while the movie waits (`ffx-host` runs the game's callback
/// there); its value is the call's result in the movie.
pub type Live = Box<dyn FnMut(&str, &[Value]) -> Value>;

/// Answers the movie's external calls: with [`Answers::live`] set, live;
/// else with the recorded answers, in order per name, keeping what it was
/// asked.
#[derive(Default)]
pub struct Answers {
    pub queued: HashMap<String, VecDeque<Option<String>>>,
    pub asked: Vec<CallOut>,
    pub live: Option<Live>,
}

struct Provider(Rc<RefCell<Answers>>);

impl ExternalInterfaceProvider for Provider {
    fn call_method(&self, _context: &mut UpdateContext<'_>, name: &str, args: &[Value]) -> Value {
        let mut a = self.0.borrow_mut();
        if let Some(live) = a.live.as_mut() {
            return live(name, args);
        }
        a.asked.push(CallOut {
            name: name.to_owned(),
            args: args.to_vec(),
        });
        match a.queued.get_mut(name).and_then(VecDeque::pop_front) {
            Some(Some(s)) => Value::String(s),
            _ => Value::Undefined,
        }
    }

    fn on_callback_available(&self, _name: &str) {}

    fn get_id(&self) -> Option<String> {
        None
    }
}

/// Converts a recorded Iggy argument to Ruffle's external value.
#[must_use]
pub fn to_value(a: &Arg) -> Value {
    match a {
        Arg::Undefined | Arg::Other(_) => Value::Undefined,
        Arg::Bool(b) => Value::Bool(*b),
        Arg::Number(n) => Value::Number(*n),
        Arg::String(s) => Value::String(s.clone()),
        Arg::Strings(v) => Value::List(v.iter().cloned().map(Value::String).collect()),
    }
}

/// The font the game's movies draw their text with: a movie's first
/// `DefineFont2`/`DefineFont3`, read once. `box.swf`'s is 2.6 MB, 7,646
/// glyphs: reading the whole movie again for each new stage took about
/// 20 ms, reading the tag 30-40 ms (this machine), every create.
pub struct GameFont {
    /// The tag, its name left empty (the name is borrowed from the movie's
    /// bytes; each registration names its copy).
    font: swf::Font<'static>,
    /// The tag's own name.
    original: String,
}

impl GameFont {
    /// Takes the first `DefineFont2`/`DefineFont3` of `box.swf` (or any
    /// movie with one), scanning the top-level tags' headers only.
    ///
    /// # Errors
    /// Not a SWF, or no such tag (a movie without a font: no stage is
    /// built to find out), or a tag that does not read.
    pub fn from_swf(data: &[u8]) -> Result<Self> {
        let swf = swf::decompress_swf(data).map_err(|e| anyhow!("{e}"))?;
        let (font_version, tag) =
            find_font_tag(&swf.data).ok_or_else(|| anyhow!("no DefineFont2/3"))?;
        let f = swf::read::Reader::new(tag, swf.header.version())
            .read_define_font_2(font_version)
            .map_err(|e| anyhow!("{e}"))?;
        let original = f.name.to_str_lossy(swf::UTF_8).into_owned();
        let font = swf::Font {
            version: f.version,
            id: f.id,
            name: swf::SwfStr::from_utf8_str(""),
            language: f.language,
            layout: f.layout,
            glyphs: f.glyphs,
            flags: f.flags,
        };
        Ok(Self { font, original })
    }

    fn register(&self, player: &mut Player) {
        for alias in ["$NormalFont", "$TitleFont", self.original.as_str()] {
            let mut f = self.font.clone();
            f.name = swf::SwfStr::from_utf8_str(alias);
            player.register_device_font(FontDefinition::SwfTag(f, swf::UTF_8));
        }
    }
}

/// The first top-level `DefineFont2` (48) or `DefineFont3` (75) tag of a
/// decompressed tag stream: its version and body. A tag's header is a
/// little-endian u16, the code in its top 10 bits, the length in the low
/// 6 (63: a u32 length follows); `End` (0) ends the list.
fn find_font_tag(data: &[u8]) -> Option<(u8, &[u8])> {
    let mut at = 0usize;
    loop {
        let head = u16::from_le_bytes(data.get(at..at + 2)?.try_into().ok()?);
        at += 2;
        let code = head >> 6;
        let mut len = usize::from(head & 0x3f);
        if len == 0x3f {
            len =
                usize::try_from(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?)).ok()?;
            at += 4;
        }
        let body = data.get(at..at.checked_add(len)?)?;
        match code {
            0 => return None,
            48 => return Some((2, body)),
            75 => return Some((3, body)),
            _ => at += len,
        }
    }
}

/// How Ruffle draws (the pilot's round 2 measurements).
#[derive(Debug, Clone, Default)]
pub struct Render {
    /// A Ruffle stage quality (`low`, `medium`, `high`, `best`, `8x8`,
    /// `16x16`, ...); `None` leaves Ruffle's default (high: 4x MSAA).
    pub quality: Option<String>,
    /// Supersampling: Ruffle draws at `ss` times the display size and the
    /// image is box-filtered down (`compose::box_down`); 0 or 1 for none.
    pub ss: u32,
}

/// Draws every bitmap smoothed, as Iggy does (the Ruffle fork's
/// `FORCE_BITMAP_SMOOTHING`).
pub fn force_bitmap_smoothing(on: bool) {
    ruffle_render_wgpu::FORCE_BITMAP_SMOOTHING.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// Gives every bitmap a full mip chain, sampled trilinear, as Iggy's `GDraw`
/// does (the Ruffle fork's `GDRAW_MIPMAPS`).
pub fn gdraw_mipmaps(on: bool) {
    ruffle_render_wgpu::GDRAW_MIPMAPS.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// Makes `getTimer` read the player's own clock (the sum of the tick
/// times) instead of the wall clock (the Ruffle fork's
/// `DETERMINISTIC_GET_TIMER`): the replay no longer depends on how fast it
/// runs.
pub fn deterministic_get_timer(on: bool) {
    ruffle_core::DETERMINISTIC_GET_TIMER.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// Reports `DisplayObject.width` / `height` in floating point, without
/// rounding to twips, as Iggy does (the fork's `IGGY_FLOAT_BOUNDS`): the
/// movies lay out bitmap glyphs with `x = int(x + glyph.width)`, and
/// twips-rounded widths moved text by a pixel here and there.
pub fn iggy_float_bounds(on: bool) {
    ruffle_core::IGGY_FLOAT_BOUNDS.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// Keeps the sub-pixel position of objects drawn through the bitmap cache
/// (filters: the labels' glow), as `GDraw` draws them into screen-sized
/// targets (the fork's `IGGY_FILTER_PHASE`); stock Ruffle snaps them.
pub fn iggy_filter_phase(on: bool) {
    ruffle_core::IGGY_FILTER_PHASE.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// Every `GDraw`-like switch on: mipmaps, forced smoothing, the
/// deterministic `getTimer`, float bounds and sub-pixel filter caches.
pub fn gdraw_like() {
    gdraw_mipmaps(true);
    force_bitmap_smoothing(true);
    deterministic_get_timer(true);
    iggy_float_bounds(true);
    iggy_filter_phase(true);
}

impl Render {
    /// How `GDraw` draws (WP12 measurements, docs/formats/pc-ui.md
    /// "Ruffle"): one sample (stage quality `low`; `GDraw` has no MSAA).
    #[must_use]
    pub fn gdraw() -> Self {
        Self {
            quality: Some("low".to_owned()),
            ss: 0,
        }
    }
}

/// One movie in a Ruffle player.
pub struct Stage {
    player: Arc<Mutex<Player>>,
    pub answers: Rc<RefCell<Answers>>,
    frame: FloatDuration,
    pub ticks: u64,
    size: (u32, u32),
    ss: u32,
    /// The offered functions' argument counts (`crate::abc`).
    arities: BTreeMap<String, Arity>,
    /// Where the last mouse move put the pointer (stage pixels).
    mouse: (f64, f64),
}

impl Stage {
    /// Loads `data` (the `.swf` as the game reads it) at the display size.
    ///
    /// # Errors
    /// The SWF or the renderer.
    pub fn new(
        gpu: &Gpu,
        data: &[u8],
        url: &str,
        size: (u32, u32),
        font: Option<&GameFont>,
        render: &Render,
    ) -> Result<Self> {
        let ss = render.ss.max(1);
        // a movie whose AS3 the reader can't take is played unfitted
        let arities = crate::abc::Movie::from_swf(data)
            .map(|m| m.arities())
            .unwrap_or_default();
        let full = (size.0 * ss, size.1 * ss);
        let movie = SwfMovieData::from_data(data, url.to_owned(), None, None)
            .map_err(|e| anyhow!("{e}"))?;
        let fps = f64::from(movie.frame_rate().to_f32());
        let target =
            TextureTarget::new(&gpu.descriptors.device, full).map_err(|e| anyhow!("{e}"))?;
        let renderer =
            WgpuRenderBackend::new(gpu.descriptors.clone(), target).map_err(|e| anyhow!("{e}"))?;
        let answers = Rc::new(RefCell::new(Answers::default()));
        let player = PlayerBuilder::new()
            .with_renderer(renderer)
            .with_movie(movie)
            .with_viewport_dimensions(full.0, full.1, f64::from(ss))
            .with_external_interface(Box::new(Provider(answers.clone())))
            .with_autoplay(true)
            .build();
        {
            let mut p = player.lock().map_err(|_| anyhow!("poisoned"))?;
            if let Some(font) = font {
                font.register(&mut p);
            }
            p.set_window_mode("transparent");
            if let Some(q) = &render.quality {
                let q = q.parse().map_err(|_| anyhow!("unknown quality {q}"))?;
                p.set_quality(q);
            }
            p.set_is_playing(true);
        }
        Ok(Self {
            player,
            answers,
            frame: FloatDuration::from_millis(1000.0 / fps),
            ticks: 0,
            size,
            ss,
            arities,
            mouse: (0.0, 0.0),
        })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Player> {
        match self.player.lock() {
            Ok(p) => p,
            Err(e) => e.into_inner(),
        }
    }

    /// `IggyPlayerSetDisplaySize`.
    pub fn set_size(&mut self, w: u32, h: u32) {
        if (w, h) != self.size && w > 0 && h > 0 {
            self.size = (w, h);
            let ss = self.ss;
            self.lock().set_viewport_dimensions(ViewportDimensions {
                width: w * ss,
                height: h * ss,
                scale_factor: f64::from(ss),
            });
        }
    }

    /// A mouse move (`IggyPlayerDispatchEventRS`), in display pixels.
    pub fn mouse_move(&mut self, x: i32, y: i32) {
        self.mouse = (
            f64::from(x) * f64::from(self.ss),
            f64::from(y) * f64::from(self.ss),
        );
        let (x, y) = self.mouse;
        self.lock()
            .handle_event(ruffle_core::PlayerEvent::MouseMove { x, y });
    }

    /// A mouse button pressed or let go (`IggyPlayerDispatchEventRS` of an
    /// `IggyMakeEventMouseButton` event) where the last move left the
    /// pointer, as Iggy places a button event (`iggy_w32.dll`
    /// `0x10012b38`: the player's last move position, `+0x22b8`/`+0x22bc`).
    pub fn mouse_button(&mut self, button: MouseButton, down: bool) {
        let (x, y) = self.mouse;
        let event = if down {
            ruffle_core::PlayerEvent::MouseDown {
                x,
                y,
                button,
                index: None,
            }
        } else {
            ruffle_core::PlayerEvent::MouseUp { x, y, button }
        };
        self.lock().handle_event(event);
    }

    /// The mouse wheel (`IggyMakeEventMouseWheel`): Iggy hands the game's
    /// value to AS3 as `MouseEvent.delta` unchanged (`0x10011eb0`; the
    /// game passes 120 a notch), which Ruffle's delta in lines is.
    pub fn mouse_wheel(&mut self, delta: i32) {
        self.lock()
            .handle_event(ruffle_core::PlayerEvent::MouseWheel {
                delta: ruffle_core::events::MouseWheelDelta::Lines(f64::from(delta)),
            });
    }

    /// One Flash frame (`IggyPlayerTickRS`).
    pub fn tick(&mut self) {
        let dt = self.frame;
        self.lock().tick(dt);
        self.ticks += 1;
    }

    /// `IggyPlayerCallFunctionRS`.
    pub fn call(&mut self, name: &str, args: &[Arg]) -> Value {
        let values: Vec<Value> = args.iter().map(to_value).collect();
        self.call_values(name, values)
    }

    /// `IggyPlayerCallFunctionRS` with Ruffle's values (string arrays as
    /// `Value::List`), fitted to the function's argument count.
    pub fn call_values(&mut self, name: &str, mut args: Vec<Value>) -> Value {
        if let Some(a) = self.arities.get(name) {
            a.fit(&mut args, &Value::Undefined);
        }
        self.lock().call_internal_interface(name, args)
    }

    /// The movie's image now, premultiplied RGBA rows (`width * 4` bytes
    /// each) as the renderer leaves them: what Iggy's `GDraw` blends over
    /// the scene (`ONE`, `INV_SRC_ALPHA`). Display size only (no
    /// supersampling).
    ///
    /// # Errors
    /// The renderer gave no image, or the stage is supersampled.
    pub fn capture_premultiplied(&mut self) -> Result<(u32, u32, Vec<u8>)> {
        if self.ss > 1 {
            return Err(anyhow!("a supersampled stage has no premultiplied image"));
        }
        let mut p = self.lock();
        p.render();
        let renderer =
            <dyn std::any::Any>::downcast_mut::<WgpuRenderBackend<TextureTarget>>(p.renderer_mut())
                .ok_or_else(|| anyhow!("not the wgpu texture renderer"))?;
        let device = renderer.descriptors().device.clone();
        let target = renderer.target();
        let (w, h) = (target.size.width, target.size.height);
        let info = target.buffer.as_ref().ok_or_else(|| anyhow!("no frame"))?;
        let (buffer, dims) = info.buffer.inner();
        let rows =
            ruffle_render_wgpu::utils::capture_image(&device, buffer, dims, None, |rgba, _| {
                let mut out = Vec::with_capacity(dims.height * dims.unpadded_bytes_per_row);
                for row in rgba.chunks(dims.padded_bytes_per_row as usize) {
                    out.extend_from_slice(&row[..dims.unpadded_bytes_per_row.min(row.len())]);
                }
                out
            });
        Ok((w, h, rows))
    }

    /// The movie's image now (straight alpha), as Iggy's draw would show it.
    ///
    /// # Errors
    /// The renderer gave no image.
    pub fn capture(&mut self) -> Result<image::RgbaImage> {
        let mut p = self.lock();
        p.render();
        let renderer =
            <dyn std::any::Any>::downcast_mut::<WgpuRenderBackend<TextureTarget>>(p.renderer_mut())
                .ok_or_else(|| anyhow!("not the wgpu texture renderer"))?;
        let img = renderer
            .capture_frame()
            .ok_or_else(|| anyhow!("no frame"))?;
        Ok(if self.ss > 1 {
            crate::compose::box_down(&img, self.ss)
        } else {
            img
        })
    }
}

#[cfg(test)]
mod tests {
    use super::find_font_tag;

    /// A tag header: short form below 63 bytes, else the long form.
    fn tag(code: u16, body: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        if body.len() < 0x3f {
            out.extend_from_slice(
                &((code << 6) | u16::try_from(body.len()).unwrap()).to_le_bytes(),
            );
        } else {
            out.extend_from_slice(&((code << 6) | 0x3f).to_le_bytes());
            out.extend_from_slice(&u32::try_from(body.len()).unwrap().to_le_bytes());
        }
        out.extend_from_slice(body);
        out
    }

    #[test]
    fn the_first_font_tag_is_found_past_long_and_short_tags() {
        let big = vec![7u8; 300];
        let data = [
            tag(69, &[1, 0, 0, 0]),
            tag(36, &big),
            tag(75, b"font3"),
            tag(48, b"font2"),
            tag(0, &[]),
        ]
        .concat();
        assert_eq!(find_font_tag(&data), Some((3, &b"font3"[..])));
        let data = [tag(48, &big), tag(0, &[])].concat();
        assert_eq!(find_font_tag(&data), Some((2, &big[..])));
    }

    #[test]
    fn a_movie_without_a_font_or_cut_short_has_none() {
        let data = [tag(1, &[]), tag(0, &[])].concat();
        assert_eq!(find_font_tag(&data), None);
        // a length past the end, or no End at all
        let mut cut = tag(75, b"font3");
        cut.truncate(4);
        assert_eq!(find_font_tag(&cut), None);
        assert_eq!(find_font_tag(&tag(1, &[])), None);
    }
}
