//! **How long the GPU spent on a frame**, by its own clock (milestone 612).
//!
//! The shell's frame statistics time the CPU. On a phone the slow frames were the
//! presentation waiting on a GPU still busy with earlier frames, and nothing said what the
//! GPU itself was spending. This asks the GPU: an empty compute pass before the frame's
//! work writes a timestamp, another after it writes a second, and the difference, read back
//! a few frames later, is the time the GPU took to get from one to the other.
//!
//! Only where the device can: `Features::TIMESTAMP_QUERY`. Elsewhere there is no timer, and
//! nothing is asked of the device. The read-back never waits: a frame's timestamps go into
//! one of a ring of buffers, mapped when the GPU is done, and read when the CPU next looks.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// How many frames' timestamps can be on their way back at once.
const RING: usize = 3;
/// Two timestamps a frame, eight bytes each.
const FRAME_BYTES: u64 = 16;

struct Slot {
    resolve: wgpu::Buffer,
    readback: wgpu::Buffer,
    /// Set by the map callback when the read-back buffer can be read.
    ready: Arc<AtomicBool>,
    /// Whether this slot holds a frame that has not been read.
    pending: bool,
}

/// The GPU's own timing of each frame.
pub(crate) struct GpuTimer {
    queries: wgpu::QuerySet,
    slots: Vec<Slot>,
    next: usize,
    /// Nanoseconds per timestamp tick.
    period: f32,
    /// The latest frame read back, in milliseconds.
    last_ms: Option<f32>,
}

impl GpuTimer {
    /// A timer, when the device was created with `TIMESTAMP_QUERY`.
    pub(crate) fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Option<Self> {
        if !device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            return None;
        }
        let queries = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("frus.gpu_timer.queries"),
            ty: wgpu::QueryType::Timestamp,
            count: (RING * 2) as u32,
        });
        let slots = (0..RING)
            .map(|_| Slot {
                resolve: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("frus.gpu_timer.resolve"),
                    size: FRAME_BYTES,
                    usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                }),
                readback: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("frus.gpu_timer.readback"),
                    size: FRAME_BYTES,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
                ready: Arc::new(AtomicBool::new(false)),
                pending: false,
            })
            .collect();
        Some(Self {
            queries,
            slots,
            next: 0,
            period: queue.get_timestamp_period(),
            last_ms: None,
        })
    }

    /// The latest frame the GPU has finished, in milliseconds.
    pub(crate) fn last_ms(&self) -> Option<f32> {
        self.last_ms
    }

    /// Reads back whatever frames the GPU has finished, without waiting for any.
    fn collect(&mut self, device: &wgpu::Device) {
        let _ = device.poll(wgpu::PollType::Poll);
        for slot in &mut self.slots {
            if !(slot.pending && slot.ready.load(Ordering::Acquire)) {
                continue;
            }
            if let Ok(view) = slot.readback.get_mapped_range(..) {
                let start = u64::from_le_bytes(view[0..8].try_into().unwrap_or([0; 8]));
                let end = u64::from_le_bytes(view[8..16].try_into().unwrap_or([0; 8]));
                drop(view);
                if end > start {
                    self.last_ms = Some((end - start) as f32 * self.period / 1_000_000.0);
                }
            }
            slot.readback.unmap();
            slot.ready.store(false, Ordering::Release);
            slot.pending = false;
        }
    }

    /// The timestamp before a frame's work: submitted on its own, ahead of it. `None` when
    /// every slot is still on its way back, and this frame goes untimed.
    pub(crate) fn begin(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) -> Option<usize> {
        self.collect(device);
        let at = self.next;
        if self.slots[at].pending {
            return None;
        }
        self.next = (self.next + 1) % RING;
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("frus.gpu_timer.begin"),
        });
        mark(&mut encoder, &self.queries, (at * 2) as u32);
        queue.submit(std::iter::once(encoder.finish()));
        Some(at)
    }

    /// The timestamp after the frame's work, and the read-back of the two.
    pub(crate) fn end(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, at: usize) {
        let slot = &mut self.slots[at];
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("frus.gpu_timer.end"),
        });
        mark(&mut encoder, &self.queries, (at * 2 + 1) as u32);
        let first = (at * 2) as u32;
        encoder.resolve_query_set(&self.queries, first..first + 2, &slot.resolve, 0);
        encoder.copy_buffer_to_buffer(&slot.resolve, 0, &slot.readback, 0, FRAME_BYTES);
        queue.submit(std::iter::once(encoder.finish()));
        let ready = slot.ready.clone();
        slot.readback
            .map_async(wgpu::MapMode::Read, .., move |result| {
                if result.is_ok() {
                    ready.store(true, Ordering::Release);
                }
            });
        slot.pending = true;
    }
}

/// An empty compute pass that writes timestamp `index`: when the GPU reached this point.
fn mark(encoder: &mut wgpu::CommandEncoder, queries: &wgpu::QuerySet, index: u32) {
    let _pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: Some("frus.gpu_timer.mark"),
        timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
            query_set: queries,
            beginning_of_pass_write_index: Some(index),
            end_of_pass_write_index: None,
        }),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A device with the GPU's clock, where the machine has one.
    fn timed_device() -> Option<(wgpu::Device, wgpu::Queue)> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
            force_fallback_adapter: false,
            ..Default::default()
        }))
        .ok()?;
        if !adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            return None;
        }
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("frus.gpu_timer.test.device"),
            required_features: wgpu::Features::TIMESTAMP_QUERY,
            required_limits: wgpu::Limits::downlevel_defaults(),
            memory_hints: wgpu::MemoryHints::default(),
            ..Default::default()
        }))
        .ok()
    }

    /// **No clock, no timer**: a device made without the feature asks nothing of it.
    #[test]
    fn a_device_without_a_clock_has_no_timer() {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
            eprintln!("no GPU adapter available: test skipped");
            return;
        };
        let Ok((device, queue)) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
        else {
            return;
        };
        assert!(GpuTimer::new(&device, &queue).is_none());
    }

    /// **Frames timed by the GPU come back**, a few frames behind, as a duration no
    /// smaller than nothing; and a frame is never held up waiting for one.
    #[test]
    fn frames_timed_by_the_gpu_come_back() {
        let Some((device, queue)) = timed_device() else {
            eprintln!("no GPU clock on this machine: test skipped");
            return;
        };
        let mut timer = GpuTimer::new(&device, &queue).expect("a timer");
        assert_eq!(timer.last_ms(), None, "nothing has been timed yet");
        for _ in 0..20 {
            if let Some(at) = timer.begin(&device, &queue) {
                timer.end(&device, &queue, at);
            }
            let _ = device.poll(wgpu::PollType::wait_indefinitely());
        }
        // One more look, for the last frame's read-back.
        let _ = timer.begin(&device, &queue);
        let ms = timer.last_ms().expect("a frame came back");
        assert!((0.0..1000.0).contains(&ms), "{ms} ms");
    }
}
