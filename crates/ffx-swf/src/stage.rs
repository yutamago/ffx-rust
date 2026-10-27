//! One game Flash screen played by Ruffle, headless (wgpu, offscreen):
//! what Iggy's player did for `FFX.exe`, mapped onto Ruffle's API
//! (`docs/formats/pc-ui.md`, "Iggy calls" and "Ruffle").
//!
//! - `IggyPlayerCallFunctionRS(name, args)` calls an AS3 function the movie
//!   registered with `ExternalInterface.addCallback`: Ruffle's
//!   `Player::call_internal_interface`.
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
use std::collections::{HashMap, VecDeque};
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

use crate::log::Arg;

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

/// Answers the movie's external calls with the recorded answers, in order
/// per name, and keeps what it was asked.
#[derive(Default)]
pub struct Answers {
    pub queued: HashMap<String, VecDeque<Option<String>>>,
    pub asked: Vec<CallOut>,
}

struct Provider(Rc<RefCell<Answers>>);

impl ExternalInterfaceProvider for Provider {
    fn call_method(&self, _context: &mut UpdateContext<'_>, name: &str, args: &[Value]) -> Value {
        let mut a = self.0.borrow_mut();
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

/// The font the game's movies draw their text with.
pub struct GameFont {
    swf: swf::SwfBuf,
}

impl GameFont {
    /// Takes the first `DefineFont3` of `box.swf` (or any movie with one).
    ///
    /// # Errors
    /// Not a SWF.
    pub fn from_swf(data: &[u8]) -> Result<Self> {
        let swf = swf::decompress_swf(data).map_err(|e| anyhow!("{e}"))?;
        Ok(Self { swf })
    }

    fn register(&self, player: &mut Player) -> Result<String> {
        let parsed = swf::parse_swf(&self.swf).map_err(|e| anyhow!("{e}"))?;
        let font = parsed
            .tags
            .iter()
            .find_map(|t| match t {
                swf::Tag::DefineFont2(f) => Some(f),
                _ => None,
            })
            .ok_or_else(|| anyhow!("no DefineFont2/3"))?;
        let original = font.name.to_str_lossy(swf::UTF_8).into_owned();
        for alias in ["$NormalFont", "$TitleFont", original.as_str()] {
            let mut f = (**font).clone();
            f.name = swf::SwfStr::from_utf8_str(alias);
            player.register_device_font(FontDefinition::SwfTag(f, swf::UTF_8));
        }
        Ok(original)
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
/// `FORCE_BITMAP_SMOOTHING`, feature `ruffle-fork`).
///
/// # Errors
/// Built against stock Ruffle.
pub fn force_bitmap_smoothing(on: bool) -> Result<()> {
    #[cfg(feature = "ruffle-fork")]
    {
        ruffle_render_wgpu::FORCE_BITMAP_SMOOTHING.store(on, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    #[cfg(not(feature = "ruffle-fork"))]
    {
        if on {
            Err(anyhow!(
                "forced smoothing needs the Ruffle fork (feature ruffle-fork)"
            ))
        } else {
            Ok(())
        }
    }
}

/// Gives every bitmap a full mip chain, sampled trilinear, as Iggy's `GDraw`
/// does (the Ruffle fork's `GDRAW_MIPMAPS`, feature `ruffle-fork`).
///
/// # Errors
/// Built against stock Ruffle.
pub fn gdraw_mipmaps(on: bool) -> Result<()> {
    #[cfg(feature = "ruffle-fork")]
    {
        ruffle_render_wgpu::GDRAW_MIPMAPS.store(on, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    #[cfg(not(feature = "ruffle-fork"))]
    {
        if on {
            Err(anyhow!(
                "mipmaps need the Ruffle fork (feature ruffle-fork)"
            ))
        } else {
            Ok(())
        }
    }
}

/// Makes `getTimer` read the player's own clock (the sum of the tick
/// times) instead of the wall clock (the Ruffle fork's
/// `DETERMINISTIC_GET_TIMER`, feature `ruffle-fork`): the replay no longer
/// depends on how fast it runs.
///
/// # Errors
/// Built against stock Ruffle.
pub fn deterministic_get_timer(on: bool) -> Result<()> {
    #[cfg(feature = "ruffle-fork")]
    {
        ruffle_core::DETERMINISTIC_GET_TIMER.store(on, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    #[cfg(not(feature = "ruffle-fork"))]
    {
        if on {
            Err(anyhow!(
                "a deterministic getTimer needs the Ruffle fork (feature ruffle-fork)"
            ))
        } else {
            Ok(())
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
                font.register(&mut p)?;
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
        self.lock()
            .handle_event(ruffle_core::PlayerEvent::MouseMove {
                x: f64::from(x) * f64::from(self.ss),
                y: f64::from(y) * f64::from(self.ss),
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
        self.lock().call_internal_interface(name, values)
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
