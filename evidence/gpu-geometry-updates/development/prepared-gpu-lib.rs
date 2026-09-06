//! Disposable GPU resources consuming immutable evaluated snapshots.
mod media;
mod raster;
mod sequence;
mod upload;
pub use upload::{Kind as GeometryUploadKind, Report as GeometryUploadReport, Statistics as GeometryUploadStatistics};
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
struct FrameTarget<'a> {
    buffer: &'a wgpu::Buffer,
    weight: f32,
    accumulate: bool,
    preserve_passes: bool,
    readback: bool,
}
struct GeometryCache {
    buffer: wgpu::Buffer,
    shadow: Vec<u8>,
}
pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    alpha_pipeline: Option<wgpu::ComputePipeline>,
    surface_pipeline: Option<wgpu::ComputePipeline>,
    dielectric_pipeline: Option<wgpu::ComputePipeline>,
    media_pipeline: Option<wgpu::ComputePipeline>,
    pub capabilities: Capabilities,
    lost: Arc<AtomicBool>,
    geometry_cache: Option<GeometryCache>,
    geometry_upload_statistics: GeometryUploadStatistics,
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
            for (index, p) in tri.positions.into_iter().enumerate() {
                let color_offset = if index == 0 && tri.colors.is_some() {
                    (11 + 2 * tri.uv_sets.len()) as f32
                } else {
                    0.
                };
                data.push(point(p, color_offset));
            }
            data.push([tri.uv[0].x, tri.uv[0].y, tri.uv[1].x, tri.uv[1].y]);
            data.push([
                tri.uv[2].x,
                tri.uv[2].y,
                f32::from(tri.normals.is_some()),
                f32::from(tri.tangents.is_some()),
            ]);
            for n in tri.normals.unwrap_or([glam::DVec3::ZERO; 3]) {
                data.push(point(n, 0.));
            }
            for t in tri.tangents.unwrap_or([glam::DVec4::ZERO; 3]) {
                data.push(t.as_vec4().to_array());
            }
            for uv in &tri.uv_sets {
                data.push([uv[0].x, uv[0].y, uv[1].x, uv[1].y]);
                data.push([uv[2].x, uv[2].y, 0., 0.]);
            }
            if let Some(colors) = tri.colors {
                data.extend(colors.map(|c| c.to_array()));
            }
        }
        data[address] = point(n.bounds.min, n.items.len() as f32);
        data[address + 1] = point(n.bounds.max, first as f32);
        data[address + 2] = [
            (base + escape * 3) as f32,
            (11 + 2 * g.uv_attributes.len() + if g.color_attribute.is_some() { 3 } else { 0 })
                as f32,
            0.,
            0.,
        ];
        escape
    }
    node(g, 0, base, data);
    base
}
pub struct Packed {
    pub geometry: Vec<[f32; 4]>,
    pub texels: Vec<[f32; 4]>,
    pub instances: Vec<[f32; 4]>,
    pub params: Vec<[f32; 4]>,
}
pub fn pack(scene: &Scene, s: &Settings, start_sample: u32) -> Result<Packed> {
    scene.validate_geometry_bindings()?;
    s.validate()?;
    if start_sample
        .checked_add(s.samples)
        .is_none_or(|v| v > 16777216)
    {
        return Err(Error::new("budget", "GPU sample index profile exceeded"));
    }
    let origin = glam::DVec3::from_array(s.camera.position);
    let media = media::pack(scene, s, origin)?;
    let mut geometry = vec![];
    let mut instances = vec![];
    let mut shared = BTreeMap::new();
    let mut maps = BTreeMap::new();
    let texel_count: usize = scene
        .images
        .values()
        .flat_map(|p| &p.levels)
        .map(|l| l.rgba.len())
        .sum();
    if texel_count > 16777216 {
        return Err(Error::new(
            "budget",
            "PBR sampled textures exceed 16 million mip texels",
        ));
    }
    let extended = scene.instances.iter().any(|i| {
        i.material
            .pbr
            .as_ref()
            .is_some_and(|p| p.advanced.is_some())
    });
    let surface_models = scene.instances.iter().any(|i| {
        i.material
            .pbr
            .as_ref()
            .and_then(|p| p.advanced.as_ref())
            .is_some_and(|a| {
                matches!(
                    a.model,
                    render_core::scattering::Model::Conductor { .. }
                        | render_core::scattering::Model::Coated { .. }
                )
            })
    });
    let dielectric = scene.instances.iter().any(|i| {
        i.material
            .pbr
            .as_ref()
            .and_then(|p| p.advanced.as_ref())
            .is_some_and(|a| matches!(a.model, render_core::scattering::Model::Dielectric { .. }))
    });
    let alpha_images: std::collections::BTreeSet<_> = scene
        .instances
        .iter()
        .filter_map(|i| {
            let p = i.material.pbr.as_ref()?;
            let a = p.advanced.as_ref()?;
            if matches!(a.opacity, render_core::scattering::Opacity::Opaque) {
                return None;
            }
            p.base_color.as_ref().map(|b| (b.image.clone(), b.role))
        })
        .collect();
    let mut texels = Vec::with_capacity(texel_count.div_ceil(2));
    let mut texel_index = 0usize;
    for (key, pyramid) in &scene.images {
        let header = geometry.len();
        geometry.resize(header + pyramid.levels.len(), [0.; 4]);
        for (j, level) in pyramid.levels.iter().enumerate() {
            geometry[header + j] = [
                texel_index as f32,
                level.width as f32,
                level.height as f32,
                0.,
            ];
            for pixel in &level.rgba {
                let pair = |a, b| {
                    f32::from_bits(
                        u32::from(half::f16::from_f32(a).to_bits())
                            | (u32::from(half::f16::from_f32(b).to_bits()) << 16),
                    )
                };
                if texel_index.is_multiple_of(2) {
                    texels.push([0.; 4]);
                }
                let record = texels.last_mut().expect("allocated texel");
                let offset = (texel_index % 2) * 2;
                record[offset] = pair(pixel[0], pixel[1]);
                record[offset + 1] = pair(pixel[2], pixel[3]);
                texel_index += 1;
            }
        }
        if alpha_images.contains(key) {
            // Coverage keeps the decoded base-level f32 alpha. Color/filter mips
            // retain the existing RGBA16 profile and opaque packing is unchanged.
            geometry[header][3] = (geometry.len() + 1) as f32;
            for pixels in pyramid.levels[0].rgba.chunks(4) {
                let mut row = [0.; 4];
                for (i, pixel) in pixels.iter().enumerate() {
                    row[i] = pixel[3];
                }
                geometry.push(row);
            }
        }
        maps.insert(key, (header, pyramid.levels.len()));
    }
    if texels.is_empty() {
        texels.push([0.; 4]);
    }
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
        instances.push(point(
            inst.inverse.transform_point3(origin),
            inst.transform.matrix3.determinant().signum() as f32,
        ));
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
        let surface = inst.material.pbr.as_ref();
        instances.push([
            width as f32,
            height as f32,
            f32::from(surface.is_some()),
            f32::from(surface.is_none_or(|p| {
                p.double_sided
                    || p.advanced.as_ref().is_some_and(|a| {
                        matches!(a.model, render_core::scattering::Model::Dielectric { .. })
                    })
            })),
        ]);
        instances.push([
            inst.material.roughness,
            inst.material.metallic,
            surface.map_or(1., |p| p.normal_scale),
            surface.map_or(1., |p| p.occlusion_strength),
        ]);
        for binding in surface
            .map_or([None; 6], |p| p.bindings())
            .into_iter()
            .take(5)
        {
            let descriptor = if let Some(binding) = binding {
                let (header, levels) = maps[&(binding.image.clone(), binding.role)];
                let offset = geometry.len();
                let sampler = &binding.sampler;
                use render_core::textures::{Filter, MinFilter, Wrap};
                let wrap = |w| match w {
                    Wrap::Repeat => 0.,
                    Wrap::Clamp => 1.,
                    Wrap::Mirror => 2.,
                };
                let min = match sampler.min {
                    MinFilter::Nearest => 0.,
                    MinFilter::Linear => 1.,
                    MinFilter::NearestMipNearest => 2.,
                    MinFilter::LinearMipNearest => 3.,
                    MinFilter::NearestMipLinear => 4.,
                    MinFilter::LinearMipLinear => 5.,
                };
                geometry.push([
                    header as f32,
                    levels as f32,
                    wrap(sampler.wrap_s),
                    wrap(sampler.wrap_t),
                ]);
                geometry.push([f32::from(sampler.mag == Filter::Linear), min, 0., 0.]);
                offset as f32
            } else {
                -1.
            };
            let uv_slot = binding.and_then(|b| b.uv_attribute).map_or(0, |id| {
                inst.geometry
                    .uv_attributes
                    .iter()
                    .position(|v| *v == id)
                    .expect("validated UV binding")
                    + 1
            });
            instances.push([descriptor, uv_slot as f32, 0., 0.]);
        }
        use render_core::scattering::Opacity;
        let opacity = surface
            .and_then(|p| p.advanced.as_ref())
            .map(|a| &a.opacity);
        let mut coverage = match opacity {
            None | Some(Opacity::Opaque) => [0.; 4],
            Some(Opacity::Blend { factor }) => [2., *factor as f32, 0., 0.],
            Some(Opacity::Mask { factor, cutoff }) => {
                // Compile the CPU f64 multiplication predicate into the first
                // accepted f32 alpha. Dividing cutoff/factor can round an exact
                // equality upward, and is not equivalent for subnormal factors.
                // Positive f32 bit patterns are monotone; this takes <=30 steps.
                if *cutoff == 0. {
                    [0.; 4]
                } else if *factor == 0. || cutoff > factor {
                    [1., 0., 2., 0.]
                } else {
                    let mut low = 0_u32;
                    let mut high = 1_f32.to_bits();
                    while low < high {
                        let middle = low + (high - low) / 2;
                        if f64::from(f32::from_bits(middle)) * factor >= *cutoff {
                            high = middle;
                        } else {
                            low = middle + 1;
                        }
                    }
                    [1., 0., f32::from_bits(low), 0.]
                }
            }
        };
        use render_core::scattering::Model;
        match surface.and_then(|p| p.advanced.as_ref()).map(|a| &a.model) {
            Some(Model::Dielectric { ior }) => {
                coverage[3] = (geometry.len() + 1) as f32;
                geometry.push([3., 0., *ior as f32, 0.]);
                geometry.push([0.; 4]);
            }
            Some(Model::Conductor { eta, k }) => {
                coverage[3] = (geometry.len() + 1) as f32;
                geometry.push([1., 0., 0., 0.]);
                // Compile the same f64 optical-constant F0 as the CPU model;
                // the GPU performs subsequent Schlick/GGX arithmetic in f32.
                let f0: [f32; 3] = std::array::from_fn(|j| {
                    (((eta[j] - 1.).powi(2) + k[j] * k[j]) / ((eta[j] + 1.).powi(2) + k[j] * k[j]))
                        as f32
                });
                geometry.push([f0[0], f0[1], f0[2], 0.]);
            }
            Some(Model::Coated {
                weight,
                ior,
                roughness,
            }) => {
                coverage[3] = (geometry.len() + 1) as f32;
                geometry.push([2., *weight as f32, *ior as f32, *roughness as f32]);
                geometry.push([0.; 4]);
            }
            _ => {}
        }
        instances.push(coverage);
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
    let (mode, xmag, ymag, aspect, near, far) = match s.camera.lens {
        Some(render_core::cameras::Lens::Perspective {
            aspect_ratio,
            near,
            far,
            ..
        }) => (
            1.,
            0.,
            0.,
            aspect_ratio.unwrap_or(f64::from(s.width) / f64::from(s.height)),
            near,
            far.unwrap_or(1e30),
        ),
        Some(render_core::cameras::Lens::Orthographic {
            xmag,
            ymag,
            near,
            far,
        }) => (2., xmag, ymag, 1., near.max(1e-5), far),
        None => (
            0.,
            0.,
            0.,
            f64::from(s.width) / f64::from(s.height),
            1e-5,
            1e30,
        ),
    };
    let mut params = vec![
        [
            s.width as f32,
            s.height as f32,
            s.samples as f32,
            s.max_depth as f32,
        ],
        [0., 0., 0., scene.instances.len() as f32],
        point(forward, (s.camera.fov() / 2.).tan() as f32),
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
        [mode, xmag as f32, ymag as f32, aspect as f32],
        [
            near as f32,
            far as f32,
            if extended {
                16.
            } else if scene.instances.iter().any(|i| i.material.pbr.is_some()) {
                3.
            } else {
                2.
            },
            if media.is_some() {
                4.
            } else if dielectric {
                3.
            } else if surface_models {
                2.
            } else {
                f32::from(extended)
            },
        ],
    ];
    params.push([1., 0., 0., 0.]);
    if let Some(media) = media {
        if geometry.len() > 16_777_216 {
            return Err(Error::new(
                "precision",
                "GPU medium offset exceeds exact f32 indexing",
            ));
        }
        params.push([
            geometry.len() as f32,
            media.count as f32,
            media.max_corner_error as f32,
            0.,
        ]);
        geometry.extend(media.rows);
    }
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
        texels,
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
            alpha_pipeline: None,
            surface_pipeline: None,
            dielectric_pipeline: None,
            media_pipeline: None,
            capabilities,
            lost,
            geometry_cache: None,
            geometry_upload_statistics: GeometryUploadStatistics::default(),
            geometry_uploads: 0,
        })
    }
    async fn ensure_alpha_pipeline(&mut self) -> Result<()> {
        if self.alpha_pipeline.is_some() {
            return Ok(());
        }
        let source = render_kernel::path::alpha_kernel().generate()?;
        self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Rust-generated Principled coverage traversal"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("portable alpha path tracing"),
                layout: None,
                module: &module,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });
        let validation = self.device.pop_error_scope().await;
        let allocation = self.device.pop_error_scope().await;
        if let Some(error) = validation.or(allocation) {
            return Err(fail("gpu_validation", error));
        }
        self.alpha_pipeline = Some(pipeline);
        Ok(())
    }
    async fn ensure_surface_pipeline(&mut self) -> Result<()> {
        if self.surface_pipeline.is_some() {
            return Ok(());
        }
        let source = render_kernel::path::surface_kernel().generate()?;
        self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Rust-generated conductor and coat traversal"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("portable surface path tracing"),
                layout: None,
                module: &module,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });
        let validation = self.device.pop_error_scope().await;
        let allocation = self.device.pop_error_scope().await;
        if let Some(error) = validation.or(allocation) {
            return Err(fail("gpu_validation", error));
        }
        self.surface_pipeline = Some(pipeline);
        Ok(())
    }
    async fn ensure_dielectric_pipeline(&mut self) -> Result<()> {
        if self.dielectric_pipeline.is_some() {
            return Ok(());
        }
        let source = render_kernel::path::dielectric_kernel().generate()?;
        self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Rust-generated ideal dielectric traversal"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("portable dielectric path tracing"),
                layout: None,
                module: &module,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });
        let validation = self.device.pop_error_scope().await;
        let allocation = self.device.pop_error_scope().await;
        if let Some(error) = validation.or(allocation) {
            return Err(fail("gpu_validation", error));
        }
        self.dielectric_pipeline = Some(pipeline);
        Ok(())
    }
    async fn ensure_media_pipeline(&mut self) -> Result<()> {
        if self.media_pipeline.is_some() {
            return Ok(());
        }
        let source = render_kernel::path::media_kernel().generate()?;
        self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Rust-generated sparse medium traversal"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("portable media path tracing"),
                layout: None,
                module: &module,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });
        let validation = self.device.pop_error_scope().await;
        let allocation = self.device.pop_error_scope().await;
        if let Some(error) = validation.or(allocation) {
            return Err(fail("gpu_validation", error));
        }
        self.media_pipeline = Some(pipeline);
        Ok(())
    }
    /// Disposable cache observations, including work on later-cancelled frames.
    /// Host packing remains global; transfer counts do not imply CPU locality.
    pub fn geometry_upload_statistics(&self) -> GeometryUploadStatistics {
        let mut statistics = self.geometry_upload_statistics.clone();
        statistics.retained_shadow_bytes = self.geometry_cache.as_ref().map_or(0, |c| c.shadow.len() as u64);
        statistics
    }
    pub fn destroy(&mut self) {
        self.lost.store(true, Ordering::Release);
        self.geometry_cache = None;
        self.alpha_pipeline = None;
        self.surface_pipeline = None;
        self.dielectric_pipeline = None;
        self.media_pipeline = None;
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
        let values = self
            .render_pixels(scene, s, start_sample, cancelled, submitted, None)
            .await?;
        self.image_from_values(scene, s, start_sample, values)
    }
    async fn render_pixels(
        &mut self,
        scene: &Scene,
        s: &Settings,
        start_sample: u32,
        cancelled: &AtomicBool,
        submitted: impl FnOnce(),
        target: Option<FrameTarget<'_>>,
    ) -> Result<Vec<f32>> {
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
        let mut p = pack(scene, s, start_sample)?;
        if let Some(t) = &target {
            p.params[10] = [
                t.weight,
                f32::from(t.accumulate),
                f32::from(t.preserve_passes),
                0.,
            ];
        }
        let output_size = u64::from(s.width) * u64::from(s.height) * 32;
        let readback_size = if target.as_ref().is_some_and(|t| !t.readback) {
            4
        } else {
            output_size
        };
        let sizes = [
            p.geometry.len() as u64 * 16,
            p.instances.len() as u64 * 16,
            p.params.len() as u64 * 16,
            output_size,
            p.texels.len() as u64 * 16,
        ];
        if sizes.iter().any(|&size| {
            size > self.capabilities.max_buffer_bytes
                || size > u64::from(self.capabilities.max_storage_binding_bytes)
        }) || sizes.iter().sum::<u64>() + readback_size > s.max_bytes
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
        let media = p.params[9][3] == 4.;
        let extended = p.params[9][3] > 0. && !media;
        let surface_models = p.params[9][3] == 2.;
        let dielectric = p.params[9][3] == 3.;
        if media {
            self.ensure_media_pipeline().await?;
        } else if dielectric {
            self.ensure_dielectric_pipeline().await?;
        } else if surface_models {
            self.ensure_surface_pipeline().await?;
        } else if extended {
            self.ensure_alpha_pipeline().await?;
        }
        self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let geometry_bytes = bytes(&p.geometry);
        let plan = upload::plan(self.geometry_cache.as_ref().map(|c| c.shadow.as_slice()), &geometry_bytes);
        match plan.kind {
            GeometryUploadKind::Allocate => {
                let buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("shared geometry and texels"),
                    contents: &geometry_bytes,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                });
                self.geometry_cache = Some(GeometryCache { buffer, shadow: geometry_bytes });
                self.geometry_uploads += 1;
            }
            GeometryUploadKind::Patch | GeometryUploadKind::Rewrite => {
                let cache = self.geometry_cache.as_mut().expect("same-size cache");
                for range in &plan.ranges {
                    self.queue.write_buffer(&cache.buffer, range.start as u64, &geometry_bytes[range.clone()]);
                }
                cache.shadow = geometry_bytes;
                self.geometry_uploads += 1;
            }
            GeometryUploadKind::Reuse => {}
        }
        self.geometry_upload_statistics.observe(&plan, p.geometry.len() * 16);
        let texture_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("PBR RGBA16 mip texels"),
                contents: &bytes(&p.texels),
                usage: wgpu::BufferUsages::STORAGE,
            });
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
        let owned_output = target.is_none().then(|| {
            self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("linear color and diagnostic passes"),
                size: output_size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            })
        });
        let output = target
            .as_ref()
            .map(|t| t.buffer)
            .or(owned_output.as_ref())
            .expect("output target");
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bounded readback or submission fence"),
            size: readback_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let geometry = &self.geometry_cache.as_ref().expect("uploaded").buffer;
        let buffers = [geometry, &instance_buffer, &params, output, &texture_buffer];
        let entries = buffers
            .iter()
            .enumerate()
            .map(|(i, b)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: b.as_entire_binding(),
            })
            .collect::<Vec<_>>();
        let pipeline = if media {
            self.media_pipeline
                .as_ref()
                .expect("created media pipeline")
        } else if dielectric {
            self.dielectric_pipeline
                .as_ref()
                .expect("created dielectric pipeline")
        } else if surface_models {
            self.surface_pipeline
                .as_ref()
                .expect("created surface pipeline")
        } else if extended {
            self.alpha_pipeline
                .as_ref()
                .expect("created alpha pipeline")
        } else {
            &self.pipeline
        };
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scene resources"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("diffuse transport"),
                timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }
        encoder.copy_buffer_to_buffer(output, 0, &staging, 0, readback_size);
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
        Ok(values)
    }
    fn image_from_values(
        &self,
        scene: &Scene,
        s: &Settings,
        start_sample: u32,
        values: Vec<f32>,
    ) -> Result<Image> {
        if values.len() != (s.width as usize * s.height as usize * 8) {
            return Err(Error::new("readback", "complete image readback required"));
        }
        let mut linear = vec![];
        let mut depth = vec![];
        let mut normals = vec![];
        let mut objects = vec![];
        for p in values.chunks_exact(8) {
            if p[3] < 0. {
                return Err(Error::new(
                    "budget",
                    "GPU alpha traversal exceeded 64 surfaces",
                ));
            }
            linear.push([p[0], p[1], p[2]]);
            depth.push(p[3]);
            normals.push([p[4], p[5], p[6]]);
            objects.push(if p[7] < 0. {
                None
            } else {
                scene.instances.get(p[7] as usize).map(|i| i.id)
            });
        }
        let mut receipt = RenderReceipt {
            revision: scene.revision.clone(),
            settings_digest: digest(&canonical(s)?),
            backend: format!(
                "gpu-f32-{}-{}/{}",
                if scene.instances.iter().any(|i| i.material.pbr.is_some()) {
                    "pbr"
                } else {
                    "diffuse"
                },
                if s.max_depth == 1 { "v0" } else { "path-v1" },
                self.capabilities.backend
            ),
            samples: s.samples,
            seed: s.seed,
            color_space: "linear-sRGB".into(),
            approximation: if s.max_depth > 1
                && scene.instances.iter().any(|i| i.material.pbr.is_some())
            {
                format!(
                    "finite depth {}; single-scattering GGX; roughness >=0.05; diffuse/GGX mixture PDF; point-light direct sampling; emissive surfaces/environment via BSDF only; primary UV-differential mipmaps, secondary LOD0; occlusion multiplies indirect throughput; RGBA16 texture precision; start sample {start_sample}; camera-relative f32",
                    s.max_depth
                )
            } else if scene.instances.iter().any(|i| i.material.pbr.is_some()) {
                format!(
                    "single-scattering GGX; roughness >=0.05; one bounce; primary UV-differential mipmaps; RGBA16 texture precision; start sample {start_sample}; camera-relative f32"
                )
            } else {
                format!(
                    "finite depth {}; two-sided Lambertian; nearest repeat textures; start sample {start_sample}; camera-relative f32",
                    s.max_depth
                )
            },
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
        if scene.instances.iter().any(|i| {
            i.material
                .pbr
                .as_ref()
                .is_some_and(|p| p.advanced.is_some())
        }) {
            receipt.backend = format!("gpu-f32-principled-alpha-v1/{}", self.capabilities.backend);
            receipt.approximation.push_str("; extended dimension stride16; stochastic BLEND and inclusive MASK; base-level f32 coverage, f32 interpolated alpha products and normalized thresholds; 64-surface continuation/visibility bounds; multiplicative transparent visibility");
        }
        if scene.instances.iter().any(|i| {
            i.material
                .pbr
                .as_ref()
                .and_then(|p| p.advanced.as_ref())
                .is_some_and(|a| {
                    matches!(
                        a.model,
                        render_core::scattering::Model::Conductor { .. }
                            | render_core::scattering::Model::Coated { .. }
                    )
                })
        }) {
            receipt.backend = format!("gpu-f32-conductor-coat-v1/{}", self.capabilities.backend);
            receipt.approximation.push_str("; conductor Schlick from compiled eta/k F0; single-interface GGX coat with Fresnel base attenuation and matching mixture PDF; no inter-layer multiple scattering");
        }
        if scene.instances.iter().any(|i| {
            i.material
                .pbr
                .as_ref()
                .and_then(|p| p.advanced.as_ref())
                .is_some_and(|a| {
                    matches!(a.model, render_core::scattering::Model::Dielectric { .. })
                })
        }) {
            receipt.backend = format!("gpu-f32-dielectric-v1/{}", self.capabilities.backend);
            receipt.approximation.push_str("; ideal dielectric Fresnel and Snell/TIR radiance eta-squared transport; geometric interface normals; f32 optical parameters and sampled branches; no directly sampled glass point-light caustics");
        }
        if !scene.media.is_empty() {
            receipt.backend = format!("gpu-f32-sparse-medium-v1/{}", self.capabilities.backend);
            receipt.approximation.push_str("; sparse RGB absorption/emission with overlapping half-open affine unit cells; f32 Beer integration and small-optical-depth series; ordinary opaque PBR only; no in-scattering; 64 occupied cells and 8388608 cell visits per dispatch; reconstructed cell-corner packing error <=1e-5m; camera-relative corners and inverse components <=1e12; nonzero packed values normal f32; no universal grazing-ray error bound");
        }
        if !scene.displacements.is_empty() {
            receipt.approximation.push_str(
                "; bounded uniform geometric displacement; see displacement conversion receipts",
            );
        }
        if !scene.conversions.is_empty() {
            receipt.approximation.push_str("; typed curves/points use bounded polygon sweeps; see evaluation conversion receipts");
        }
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
            "{}; accumulated samples 0..{}",
            next.receipt.approximation, self.next_sample
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
