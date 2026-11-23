//! Optional, sampled GPU render-pass timings. Readback never blocks the UI.
use std::{collections::VecDeque, sync::mpsc};

pub(super) struct GpuTimer {
    query: wgpu::QuerySet,
    resolved: wgpu::Buffer,
    readback: wgpu::Buffer,
    pending: Option<mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>>,
    active: bool,
    frame: u32,
    period_ns: f64,
    samples: VecDeque<f64>,
}

impl GpuTimer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Option<Self> {
        if !device.features().contains(wgpu::Features::TIMESTAMP_QUERY) { return None; }
        Some(Self {
            query: device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("Viewport GPU timing"), ty: wgpu::QueryType::Timestamp, count: 2,
            }),
            resolved: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Resolved GPU timestamps"), size: 16,
                usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }),
            readback: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Asynchronous GPU timestamp readback"), size: 16,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            pending: None, active: false, frame: 0,
            period_ns: queue.get_timestamp_period() as f64,
            samples: VecDeque::new(),
        })
    }
    pub fn begin_frame(&mut self, device: &wgpu::Device) {
        if self.pending.is_some() {
            device.poll(wgpu::Maintain::Poll);
            let result = self.pending.as_ref().unwrap().try_recv();
            match result {
                Ok(Ok(())) => {
                    let mapped = self.readback.slice(..).get_mapped_range();
                    let start = u64::from_ne_bytes(mapped[0..8].try_into().unwrap());
                    let end = u64::from_ne_bytes(mapped[8..16].try_into().unwrap());
                    if let Some(milliseconds) = elapsed_ms(start, end, self.period_ns) {
                        if self.samples.len() == 120 { self.samples.pop_front(); }
                        self.samples.push_back(milliseconds);
                    }
                    drop(mapped);
                    self.readback.unmap();
                    self.pending = None;
                }
                Ok(Err(_)) | Err(mpsc::TryRecvError::Disconnected) => {
                    self.readback.unmap();
                    self.pending = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        // One sample per 15 rendered frames; skip rather than wait if readback is busy.
        self.active = self.frame == 0 && self.pending.is_none();
        self.frame = (self.frame + 1) % 15;
    }
    pub fn writes(&self) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        self.active.then_some(wgpu::RenderPassTimestampWrites {
            query_set: &self.query, beginning_of_pass_write_index: Some(0), end_of_pass_write_index: Some(1),
        })
    }
    pub fn resolve(&self, encoder: &mut wgpu::CommandEncoder) {
        if self.active {
            encoder.resolve_query_set(&self.query, 0..2, &self.resolved, 0);
            encoder.copy_buffer_to_buffer(&self.resolved, 0, &self.readback, 0, 16);
        }
    }
    pub fn submitted(&mut self) {
        if self.active {
            let (sender, receiver) = mpsc::channel();
            self.readback.slice(..).map_async(wgpu::MapMode::Read, move |result| { let _ = sender.send(result); });
            self.pending = Some(receiver);
            self.active = false;
        }
    }
    pub fn summary(&self) -> Option<(usize, f64, f64)> {
        if self.samples.is_empty() { return None; }
        let mut samples: Vec<_> = self.samples.iter().copied().collect();
        samples.sort_by(f64::total_cmp);
        Some((samples.len(), samples[(samples.len() - 1) / 2], samples[(samples.len() - 1) * 95 / 100]))
    }
}

fn elapsed_ms(start: u64, end: u64, period_ns: f64) -> Option<f64> {
    let milliseconds = end.checked_sub(start)? as f64 * period_ns / 1_000_000.0;
    (period_ns > 0.0 && milliseconds.is_finite()).then_some(milliseconds)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn timestamp_units_and_invalid_samples_are_handled() {
        assert_eq!(elapsed_ms(100, 1100, 100.0), Some(0.1));
        assert_eq!(elapsed_ms(1100, 100, 1.0), None);
        assert_eq!(elapsed_ms(0, 100, 0.0), None);
        assert_eq!(elapsed_ms(0, 100, f64::NAN), None);
    }
    #[test]
    #[ignore = "requires a GPU with timestamp query support"]
    fn timestamp_readback_and_unsupported_device_fallback() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY, ..Default::default()
        });
        let adapter = instance.enumerate_adapters(wgpu::Backends::PRIMARY).into_iter()
            .find(|adapter| adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY))
            .expect("timestamp-capable adapter");
        eprintln!("Timestamp adapter: {:?}", adapter.get_info());
        for features in [wgpu::Features::empty(), wgpu::Features::TIMESTAMP_QUERY] {
            let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("GPU timing regression"), required_features: features,
                required_limits: wgpu::Limits::default(), memory_hints: Default::default(),
            }, None)).unwrap();
            let timer = GpuTimer::new(&device, &queue);
            if features.is_empty() { assert!(timer.is_none()); continue; }
            let mut timer = timer.unwrap();
            timer.begin_frame(&device);
            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: None, size: wgpu::Extent3d { width: 64, height: 64, depth_or_array_layers: 1 },
                mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[],
            });
            let view = target.create_view(&Default::default());
            let mut encoder = device.create_command_encoder(&Default::default());
            {
                let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view, resolve_target: None, ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None, timestamp_writes: timer.writes(), occlusion_query_set: None,
                });
            }
            timer.resolve(&mut encoder);
            queue.submit([encoder.finish()]);
            timer.submitted();
            // Blocking poll is restricted to this explicit hardware test.
            device.poll(wgpu::Maintain::Wait);
            timer.begin_frame(&device);
            let summary = timer.summary().expect("completed timestamp readback");
            assert_eq!(summary.0, 1);
            assert!(summary.1.is_finite() && summary.1 >= 0.0);
            eprintln!("GPU pass milliseconds: {}", summary.1);
        }
    }
}
