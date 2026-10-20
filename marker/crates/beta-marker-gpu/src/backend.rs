use anyhow::{Context, Result, ensure};
use std::time::{Duration, Instant};
use wgpu::util::DeviceExt;

use crate::{Raster, ScanStats};

pub(crate) struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    timestamps: Option<wgpu::QuerySet>,
    pub name: String,
}

impl Gpu {
    pub fn new() -> Result<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN | wgpu::Backends::DX12,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
            ..Default::default()
        }))
        .context("no Vulkan/DX12 GPU adapter")?;
        let info = adapter.get_info();
        ensure!(
            info.device_type != wgpu::DeviceType::Cpu,
            "software adapter is not a GPU"
        );
        let features = adapter.features() & wgpu::Features::TIMESTAMP_QUERY;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("BETA headless raster proposals"),
            required_features: features,
            ..Default::default()
        }))
        .context("request GPU device")?;
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("BETA raster collision scan"),
            source: wgpu::ShaderSource::Wgsl(include_str!("scan.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("BETA raster scan"),
            layout: None,
            module: &shader,
            entry_point: Some("scan"),
            compilation_options: Default::default(),
            cache: None,
        });
        if let Some(error) = pollster::block_on(scope.pop()) {
            anyhow::bail!("GPU shader: {error}");
        }
        let timestamps = features.contains(wgpu::Features::TIMESTAMP_QUERY).then(|| {
            device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("scan timestamps"),
                ty: wgpu::QueryType::Timestamp,
                count: 2,
            })
        });
        Ok(Self {
            device,
            queue,
            pipeline,
            timestamps,
            name: format!("{} / {:?}", info.name, info.backend),
        })
    }

    pub fn scan(
        &self,
        strip: &Raster,
        shape: &Raster,
        max_x: u32,
        first: u32,
        stats: &mut ScanStats,
    ) -> Result<Option<u32>> {
        let started = Instant::now();
        let y_count = strip.height - shape.height + 1;
        let candidates = (max_x + 1)
            .checked_mul(y_count)
            .context("candidate index overflow")?;
        let make = |label, contents: &[u32], usage| {
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(label),
                    contents: bytemuck::cast_slice(contents),
                    usage,
                })
        };
        let grid = make("strip", &strip.bits, wgpu::BufferUsages::STORAGE);
        let mask = make("piece", &shape.bits, wgpu::BufferUsages::STORAGE);
        let params = make(
            "parameters",
            &[
                strip.stride,
                shape.rows,
                shape.stride,
                y_count,
                candidates,
                first,
                0,
                0,
            ],
            wgpu::BufferUsages::UNIFORM,
        );
        let result = make(
            "best position",
            &[u32::MAX],
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("position readback"),
            size: 24,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let timestamp_buffer = self.timestamps.as_ref().map(|_| {
            self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("timestamp resolve"),
                size: 16,
                usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            })
        });
        let layout = self.pipeline.get_bind_group_layout(0);
        let bindings = [&grid, &mask, &params, &result];
        let entries: Vec<_> = bindings
            .iter()
            .enumerate()
            .map(|(i, buffer)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: buffer.as_entire_binding(),
            })
            .collect();
        let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &entries,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: self.timestamps.as_ref().map(|query_set| {
                    wgpu::ComputePassTimestampWrites {
                        query_set,
                        beginning_of_pass_write_index: Some(0),
                        end_of_pass_write_index: Some(1),
                    }
                }),
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &group, &[]);
            let groups = candidates.div_ceil(64);
            pass.dispatch_workgroups(groups.min(65_535), groups.div_ceil(65_535), 1);
        }
        encoder.copy_buffer_to_buffer(&result, 0, &readback, 0, 4);
        if let Some((queries, buffer)) = self.timestamps.as_ref().zip(timestamp_buffer.as_ref()) {
            encoder.resolve_query_set(queries, 0..2, buffer, 0);
            encoder.copy_buffer_to_buffer(buffer, 0, &readback, 8, 16);
        }
        let submission = self.queue.submit([encoder.finish()]);
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |value| {
                let _ = send.send(value);
            });
        self.device.poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(30)),
        })?;
        receive
            .recv_timeout(Duration::from_secs(1))
            .context("GPU readback callback")??;
        let view = readback.slice(..).get_mapped_range()?;
        let bytes: [u8; 4] = view[..4].try_into()?;
        let value = u32::from_le_bytes(bytes);
        if self.timestamps.is_some() {
            let begin = u64::from_le_bytes(view[8..16].try_into()?);
            let end = u64::from_le_bytes(view[16..24].try_into()?);
            let seconds = end.saturating_sub(begin) as f64
                * f64::from(self.queue.get_timestamp_period())
                * 1e-9;
            stats.gpu_kernel_s = Some(stats.gpu_kernel_s.unwrap_or_default() + seconds);
        }
        drop(view);
        readback.unmap();
        stats.dispatches += 1;
        stats.candidates += u64::from(candidates.saturating_sub(first));
        stats.scan_s += started.elapsed().as_secs_f64();
        Ok((value != u32::MAX).then_some(value))
    }
}
