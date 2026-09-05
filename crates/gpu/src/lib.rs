//! Disposable GPU resources consuming immutable evaluated snapshots.
mod raster;
use render_core::{
    Error, Result, canonical, digest,
    render::{Geometry, Image, RenderReceipt, Scene, Settings},
};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    task::Poll,
};
use wgpu::util::DeviceExt;

#[derive(Debug, Clone, Serialize)]
pub struct Capabilities {
    pub adapter: String,
    pub backend: String,
    pub max_buffer_bytes: u64,
    pub max_storage_binding_bytes: u32,
    pub max_workgroups: u32,
    pub hardware_ray_required: bool,
}
pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    pub capabilities: Capabilities,
    lost: Arc<AtomicBool>,
    geometry_cache: Option<(String, wgpu::Buffer)>,
    pub geometry_uploads: u64,
}
fn fail(code: &str, e: impl std::fmt::Display) -> Error {
    Error::new(code, e.to_string()).at("gpu")
}
fn bytes(values: &[[f32; 4]]) -> Vec<u8> {
    values
        .iter()
        .flatten()
        .flat_map(|v| v.to_le_bytes())
        .collect()
}
fn point(p: glam::DVec3, w: f32) -> [f32; 4] {
    [p.x as f32, p.y as f32, p.z as f32, w]
}
fn pack_geometry(g: &Geometry, data: &mut Vec<[f32; 4]>) -> usize {
    let base = data.len();
    data.resize(base + g.bvh.nodes.len() * 3, [0.; 4]);
    fn node(g: &Geometry, index: usize, base: usize, data: &mut Vec<[f32; 4]>) -> usize {
        let n = &g.bvh.nodes[index];
        let address = base + index * 3;
        let escape = if let Some([left, right]) = n.children {
            node(g, left, base, data);
            node(g, right, base, data)
        } else {
            index + 1
        };
        let first = data.len();
        for &t in &n.items {
            let tri = &g.triangles[t];
            for p in tri.positions {
                data.push(point(p, 0.));
            }
            data.push([tri.uv[0].x, tri.uv[0].y, tri.uv[1].x, tri.uv[1].y]);
            data.push([tri.uv[2].x, tri.uv[2].y, 0., 0.]);
        }
        data[address] = point(n.bounds.min, n.items.len() as f32);
        data[address + 1] = point(n.bounds.max, first as f32);
        data[address + 2] = [(base + escape * 3) as f32, 0., 0., 0.];
        escape
    }
    node(g, 0, base, data);
    base
}
pub struct Packed {
    pub geometry: Vec<[f32; 4]>,
    pub instances: Vec<[f32; 4]>,
    pub params: Vec<[f32; 4]>,
}
pub fn pack(scene: &Scene, s: &Settings, start_sample: u32) -> Result<Packed> {
    s.validate()?;
    if s.max_depth != 1 {
        return Err(Error::new(
            "unsupported_profile",
            "GPU diffuse v0 supports max_depth=1",
        ));
    }
    if start_sample
        .checked_add(s.samples)
        .is_none_or(|v| v > 16777216)
    {
        return Err(Error::new("budget", "GPU sample index profile exceeded"));
    }
    let origin = glam::DVec3::from_array(s.camera.position);
    let mut geometry = vec![];
    let mut instances = vec![];
    let mut shared = BTreeMap::new();
    for inst in &scene.instances {
        let root = *shared
            .entry(&inst.geometry_id)
            .or_insert_with(|| pack_geometry(&inst.geometry, &mut geometry));
        let texoffset = geometry.len();
        let (width, height) = if let Some(t) = &inst.material.texture {
            for p in &t.linear_rgb {
                geometry.push([p[0], p[1], p[2], 0.]);
            }
            (t.width, t.height)
        } else {
            (0, 0)
        };
        for c in inst.inverse.matrix3.to_cols_array_2d() {
            instances.push([c[0] as f32, c[1] as f32, c[2] as f32, 0.]);
        }
        instances.push(point(inst.inverse.transform_point3(origin), 0.));
        instances.push(point(inst.bounds.min - origin, 0.));
        instances.push(point(inst.bounds.max - origin, 0.));
        instances.push([
            inst.material.base_color[0],
            inst.material.base_color[1],
            inst.material.base_color[2],
            root as f32,
        ]);
        instances.push([
            inst.material.emission[0],
            inst.material.emission[1],
            inst.material.emission[2],
            texoffset as f32,
        ]);
        instances.push([width as f32, height as f32, 0., 0.]);
    }
    if geometry.len() > 16777216 || instances.len() > 16777216 {
        return Err(Error::new(
            "budget",
            "GPU index profile exceeds exact f32 addressing",
        ));
    }
    if geometry.is_empty() {
        geometry.push([0.; 4]);
    }
    if instances.is_empty() {
        instances.push([0.; 4]);
    }
    let (forward, right, up) = s.camera.basis()?;
    let params = vec![
        [s.width as f32, s.height as f32, s.samples as f32, 0.],
        [0., 0., 0., scene.instances.len() as f32],
        point(forward, (s.camera.vertical_fov_radians / 2.).tan() as f32),
        point(right, 0.),
        point(up, 0.),
        point(
            glam::DVec3::from_array(s.light.position) - origin,
            (s.seed & 65535) as f32,
        ),
        [
            s.light.intensity[0],
            s.light.intensity[1],
            s.light.intensity[2],
            start_sample as f32,
        ],
        [
            s.environment[0],
            s.environment[1],
            s.environment[2],
            (s.seed >> 16) as f32,
        ],
    ];
    if geometry
        .iter()
        .chain(&instances)
        .chain(&params)
        .flatten()
        .any(|v| !v.is_finite())
    {
        return Err(Error::new(
            "precision",
            "GPU conversion exceeds f32 finite range",
        ));
    }
    Ok(Packed {
        geometry,
        instances,
        params,
    })
}
impl Renderer {
    pub async fn new() -> Result<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .map_err(|e| fail("adapter_unavailable", e))?;
        let limits = adapter.limits();
        let info = adapter.get_info();
        let required = wgpu::Limits::default();
        if !required.check_limits(&limits) {
            return Err(Error::new(
                "device_limits",
                "adapter does not support baseline WebGPU limits",
            ));
        }
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("render portable GPU"),
                required_features: wgpu::Features::empty(),
                required_limits: required,
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|e| fail("device", e))?;
        let lost = Arc::new(AtomicBool::new(false));
        let flag = lost.clone();
        device.set_device_lost_callback(move |_, _| {
            flag.store(true, Ordering::Release);
        });
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let source = render_kernel::path::kernel().generate()?;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Rust-generated diffuse traversal"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("portable path tracing"),
            layout: None,
            module: &module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        if let Some(error) = device.pop_error_scope().await {
            return Err(fail("gpu_validation", error));
        }
        let limits = device.limits();
        let capabilities = Capabilities {
            adapter: info.name,
            backend: format!("{:?}", info.backend),
            max_buffer_bytes: limits.max_buffer_size,
            max_storage_binding_bytes: limits.max_storage_buffer_binding_size,
            max_workgroups: limits.max_compute_workgroups_per_dimension,
            hardware_ray_required: false,
        };
        Ok(Self {
            device,
            queue,
            pipeline,
            capabilities,
            lost,
            geometry_cache: None,
            geometry_uploads: 0,
        })
    }
    pub fn destroy(&mut self) {
        self.lost.store(true, Ordering::Release);
        self.geometry_cache = None;
        self.device.destroy();
    }
    pub fn is_lost(&self) -> bool {
        self.lost.load(Ordering::Acquire)
    }
    /// Exercise a real rejected GPU allocation without exhausting system memory.
    pub async fn allocation_fault_probe(&self) -> Result<bool> {
        self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("intentional over-limit allocation conformance"),
            size: self.capabilities.max_buffer_bytes + 4,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let rejected = self.device.pop_error_scope().await.is_some();
        buffer.destroy();
        Ok(rejected)
    }
    pub async fn render(
        &mut self,
        scene: &Scene,
        s: &Settings,
        start_sample: u32,
        cancelled: &AtomicBool,
    ) -> Result<Image> {
        self.render_observed(scene, s, start_sample, cancelled, || {})
            .await
    }
    /// Deterministic cancellation at the actual queue-submission boundary.
    pub async fn cancellation_fault_probe(&mut self, scene: &Scene, s: &Settings) -> Result<bool> {
        let cancelled = AtomicBool::new(false);
        match self
            .render_observed(scene, s, 0, &cancelled, || {
                cancelled.store(true, Ordering::Release)
            })
            .await
        {
            Err(error) if error.code == "cancelled" => Ok(true),
            Err(error) => Err(error),
            Ok(_) => Ok(false),
        }
    }
    async fn render_observed(
        &mut self,
        scene: &Scene,
        s: &Settings,
        start_sample: u32,
        cancelled: &AtomicBool,
        submitted: impl FnOnce(),
    ) -> Result<Image> {
        if cancelled.load(Ordering::Acquire) {
            return Err(Error::new(
                "cancelled",
                "GPU render cancelled before submission",
            ));
        }
        if self.is_lost() {
            return Err(Error::new(
                "device_lost",
                "recreate GPU resources from snapshot",
            ));
        }
        let p = pack(scene, s, start_sample)?;
        let output_size = u64::from(s.width) * u64::from(s.height) * 32;
        let sizes = [
            p.geometry.len() as u64 * 16,
            p.instances.len() as u64 * 16,
            p.params.len() as u64 * 16,
            output_size,
        ];
        if sizes.iter().any(|&size| {
            size > self.capabilities.max_buffer_bytes
                || size > u64::from(self.capabilities.max_storage_binding_bytes)
        }) || sizes.iter().sum::<u64>() + output_size > s.max_bytes
        {
            return Err(Error::new(
                "budget",
                "GPU allocation exceeds device or job memory limit",
            ));
        }
        let workgroups = (s.width * s.height).div_ceil(64);
        if workgroups > self.capabilities.max_workgroups {
            return Err(Error::new(
                "device_limits",
                "dispatch exceeds negotiated workgroup count",
            ));
        }
        self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let geometry_bytes = bytes(&p.geometry);
        let key = digest(&geometry_bytes);
        if self.geometry_cache.as_ref().is_none_or(|(k, _)| k != &key) {
            let b = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("shared geometry and texels"),
                    contents: &geometry_bytes,
                    usage: wgpu::BufferUsages::STORAGE,
                });
            self.geometry_cache = Some((key, b));
            self.geometry_uploads += 1;
        }
        let instance_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("instance transforms and materials"),
                contents: &bytes(&p.instances),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let params = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("render parameters"),
                contents: &bytes(&p.params),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let output = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("linear color and diagnostic passes"),
            size: output_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bounded readback"),
            size: output_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let geometry = &self.geometry_cache.as_ref().expect("uploaded").1;
        let buffers = [geometry, &instance_buffer, &params, &output];
        let entries = buffers
            .iter()
            .enumerate()
            .map(|(i, b)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: b.as_entire_binding(),
            })
            .collect::<Vec<_>>();
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scene resources"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("diffuse transport"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&output, 0, &staging, 0, output_size);
        self.queue.submit([encoder.finish()]);
        submitted();
        let state = Arc::new(Mutex::new((None, None::<std::task::Waker>)));
        let callback = state.clone();
        staging
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let mut state = callback.lock().expect("map callback");
                state.0 = Some(result);
                if let Some(w) = state.1.take() {
                    w.wake();
                }
            });
        #[cfg(not(target_arch = "wasm32"))]
        let poll_result = self
            .device
            .poll(wgpu::PollType::Wait)
            .map(|_| ())
            .map_err(|e| fail("device_poll", e));
        #[cfg(target_arch = "wasm32")]
        let poll_result: Result<()> = Ok(());
        let mapping = match poll_result {
            Ok(()) => std::future::poll_fn(|cx| {
                let mut state = state.lock().expect("map state");
                if let Some(result) = state.0.take() {
                    Poll::Ready(result)
                } else {
                    state.1 = Some(cx.waker().clone());
                    Poll::Pending
                }
            })
            .await
            .map_err(|e| fail("readback", e)),
            Err(error) => Err(error),
        };
        let validation = self.device.pop_error_scope().await;
        let allocation = self.device.pop_error_scope().await;
        if let Err(error) = mapping {
            staging.destroy();
            self.lost.store(true, Ordering::Release);
            return Err(error);
        }
        if let Some(error) = validation.or(allocation) {
            self.geometry_cache = None;
            return Err(fail("gpu_execution", error));
        }
        if cancelled.load(Ordering::Acquire) {
            staging.unmap();
            return Err(Error::new(
                "cancelled",
                "GPU work finished; cancelled result discarded",
            ));
        }
        let view = staging.slice(..).get_mapped_range();
        let values = view
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes(b.try_into().expect("f32 bytes")))
            .collect::<Vec<_>>();
        drop(view);
        staging.unmap();
        if values.iter().any(|v| !v.is_finite()) {
            return Err(Error::new("numerics", "GPU produced nonfinite output"));
        }
        let mut linear = vec![];
        let mut depth = vec![];
        let mut normals = vec![];
        let mut objects = vec![];
        for p in values.chunks_exact(8) {
            linear.push([p[0], p[1], p[2]]);
            depth.push(p[3]);
            normals.push([p[4], p[5], p[6]]);
            objects.push(if p[7] < 0. {
                None
            } else {
                scene.instances.get(p[7] as usize).map(|i| i.id)
            });
        }
        let receipt = RenderReceipt {
            revision: scene.revision.clone(),
            settings_digest: digest(&canonical(s)?),
            backend: format!("gpu-f32-diffuse-v0/{}", self.capabilities.backend),
            samples: s.samples,
            seed: s.seed,
            color_space: "linear-sRGB".into(),
            approximation: format!(
                "finite depth 1; two-sided Lambertian; nearest repeat textures; start sample {start_sample}; camera-relative f32"
            ),
            width: s.width,
            height: s.height,
            output_digest: digest(
                &linear
                    .iter()
                    .flatten()
                    .flat_map(|v| v.to_le_bytes())
                    .collect::<Vec<_>>(),
            ),
        };
        Ok(Image {
            width: s.width,
            height: s.height,
            linear,
            depth,
            normals,
            objects,
            receipt,
        })
    }
}

/// Accumulation is keyed to the complete scene/settings identity and sample sequence.
#[derive(Default)]
pub struct Progressive {
    image: Option<Image>,
    next_sample: u32,
    key: String,
}
impl Progressive {
    pub async fn step(
        &mut self,
        gpu: &mut Renderer,
        scene: &Scene,
        settings: &Settings,
        cancelled: &AtomicBool,
    ) -> Result<&Image> {
        let mut identity = settings.clone();
        identity.samples = 1;
        let key = digest(&canonical(&(scene.revision.as_str(), identity))?);
        if key != self.key {
            self.image = None;
            self.next_sample = 0;
            self.key = key;
        }
        if self
            .next_sample
            .checked_add(settings.samples)
            .is_none_or(|n| n > 65536)
        {
            return Err(Error::new("budget", "progressive sample profile exceeded"));
        }
        let mut next = gpu
            .render(scene, settings, self.next_sample, cancelled)
            .await?;
        if let Some(previous) = &self.image {
            let total = self.next_sample + settings.samples;
            for (out, old) in next.linear.iter_mut().zip(&previous.linear) {
                for k in 0..3 {
                    out[k] = (out[k] * settings.samples as f32 + old[k] * self.next_sample as f32)
                        / total as f32;
                }
            }
            next.depth = previous.depth.clone();
            next.normals = previous.normals.clone();
            next.objects = previous.objects.clone();
        }
        self.next_sample += settings.samples;
        next.receipt.samples = self.next_sample;
        let mut total_settings = settings.clone();
        total_settings.samples = self.next_sample;
        next.receipt.settings_digest = digest(&canonical(&total_settings)?);
        next.receipt.approximation = format!(
            "finite depth 1; two-sided Lambertian; nearest repeat textures; accumulated samples 0..{}; camera-relative f32",
            self.next_sample
        );
        next.receipt.output_digest = digest(
            &next
                .linear
                .iter()
                .flatten()
                .flat_map(|v| v.to_le_bytes())
                .collect::<Vec<_>>(),
        );
        self.image = Some(next);
        Ok(self.image.as_ref().expect("accumulated"))
    }
}
