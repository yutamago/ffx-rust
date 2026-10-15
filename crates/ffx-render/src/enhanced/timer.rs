//! GPU time per pass: timestamps between the frame's passes, read back a
//! few frames later (where the GPU can write them inside encoders).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Timestamps a frame can have.
const MAX: u32 = 32;
/// Frames in flight before their figures are read.
const SLOTS: usize = 3;

struct Slot {
    buffer: wgpu::Buffer,
    /// The pass names of the frame it holds; none: free.
    marks: Option<Vec<&'static str>>,
    /// Mapped and ready to read.
    ready: Arc<AtomicBool>,
    mapping: bool,
}

pub struct Timer {
    set: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    slots: Vec<Slot>,
    /// Nanoseconds per tick.
    period: f32,
    marks: Vec<&'static str>,
    /// The slot this frame writes, if one is free.
    slot: Option<usize>,
    next: usize,
    /// The last frame read: each pass and its milliseconds, and the
    /// whole.
    pub last: Vec<(&'static str, f32)>,
    pub total: f32,
}

impl Timer {
    /// A timer, where the device can write timestamps in encoders.
    #[must_use]
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Option<Self> {
        if !device.features().contains(
            wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS,
        ) {
            return None;
        }
        let size = u64::from(MAX) * 8;
        let set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("gpu timer"),
            ty: wgpu::QueryType::Timestamp,
            count: MAX,
        });
        let resolve = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gpu timer resolve"),
            size,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let slots = (0..SLOTS)
            .map(|_| Slot {
                buffer: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("gpu timer readback"),
                    size,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
                marks: None,
                ready: Arc::new(AtomicBool::new(false)),
                mapping: false,
            })
            .collect();
        Some(Self {
            set,
            resolve,
            slots,
            period: queue.get_timestamp_period(),
            marks: Vec::new(),
            slot: None,
            next: 0,
            last: Vec::new(),
            total: 0.0,
        })
    }

    /// A new frame: it is timed when a slot is free.
    pub fn begin(&mut self, enc: &mut wgpu::CommandEncoder) {
        self.marks.clear();
        let k = self.next % SLOTS;
        self.slot = self.slots[k].marks.is_none().then_some(k);
        if self.slot.is_some() {
            self.next += 1;
            self.mark(enc, "");
        }
    }

    /// The pass `name` ends here.
    pub fn mark(&mut self, enc: &mut wgpu::CommandEncoder, name: &'static str) {
        if self.slot.is_some() && (self.marks.len() as u32) < MAX {
            enc.write_timestamp(&self.set, self.marks.len() as u32);
            self.marks.push(name);
        }
    }

    /// The frame's figures go to their slot (in its last encoder).
    pub fn finish(&mut self, enc: &mut wgpu::CommandEncoder) {
        let Some(k) = self.slot else {
            return;
        };
        let n = self.marks.len() as u32;
        if n < 2 {
            self.slot = None;
            return;
        }
        enc.resolve_query_set(&self.set, 0..n, &self.resolve, 0);
        enc.copy_buffer_to_buffer(&self.resolve, 0, &self.slots[k].buffer, 0, u64::from(n) * 8);
        self.slots[k].marks = Some(std::mem::take(&mut self.marks));
    }

    /// After the frame's last submission: its figures are read when the
    /// GPU is done with them.
    pub fn submitted(&mut self) {
        let Some(k) = self.slot.take() else {
            return;
        };
        let s = &mut self.slots[k];
        if s.marks.is_none() || s.mapping {
            return;
        }
        s.mapping = true;
        let ready = s.ready.clone();
        s.buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            if r.is_ok() {
                ready.store(true, Ordering::Release);
            }
        });
    }

    /// Reads what the GPU has finished.
    pub fn poll(&mut self, device: &wgpu::Device) {
        let _ = device.poll(wgpu::PollType::Poll);
        for s in &mut self.slots {
            if !s.ready.swap(false, Ordering::Acquire) {
                continue;
            }
            let marks = s.marks.take().unwrap_or_default();
            {
                let Ok(data) = s.buffer.slice(..).get_mapped_range() else {
                    s.buffer.unmap();
                    s.mapping = false;
                    continue;
                };
                let ticks: Vec<u64> = data
                    .as_chunks::<8>()
                    .0
                    .iter()
                    .take(marks.len())
                    .map(|b| u64::from_le_bytes(*b))
                    .collect();
                let ms = |a: u64, b: u64| b.saturating_sub(a) as f32 * self.period / 1e6;
                self.last = marks
                    .iter()
                    .skip(1)
                    .zip(ticks.windows(2))
                    .map(|(n, w)| (*n, ms(w[0], w[1])))
                    .collect();
                self.total = match (ticks.first(), ticks.last()) {
                    (Some(&a), Some(&b)) => ms(a, b),
                    _ => 0.0,
                };
            }
            s.buffer.unmap();
            s.mapping = false;
        }
    }
}
