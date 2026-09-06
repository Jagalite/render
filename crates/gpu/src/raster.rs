use super::*;
impl Renderer {
    /// Approximate hardware raster preview: flat diffuse shading, no shadows or textures.
    pub async fn raster_preview(&self, scene: &Scene, s: &Settings) -> Result<Vec<[u8; 4]>> {
        s.validate()?;
        if scene.instances.iter().any(|i| i.material.pbr.is_some()) || s.camera.lens.is_some() {
            return Err(Error::new(
                "unsupported_profile",
                "flat raster preview does not support PBR or authored lenses; use portable path renderer",
            ));
        }
        if self.is_lost() {
            return Err(Error::new("device_lost", "raster device lost"));
        }
        let origin = glam::DVec3::from_array(s.camera.position);
        let (forward, right, up) = s.camera.basis()?;
        let half = (s.camera.vertical_fov_radians * 0.5).tan();
        let aspect = s.width as f64 / s.height as f64;
        let near = 0.01;
        let far = 10000.;
        let mut vertices = Vec::new();
        for instance in &scene.instances {
            for tri in &instance.geometry.triangles {
                let world = tri
                    .positions
                    .map(|p| instance.transform.transform_point3(p));
                let center = (world[0] + world[1] + world[2]) / 3.;
                let normal = (world[1] - world[0]).cross(world[2] - world[0]).normalize();
                let light = (glam::DVec3::from_array(s.light.position) - center).normalize();
                let shade = 0.2 + 0.8 * normal.dot(light).abs() as f32;
                for p in world {
                    let delta = p - origin;
                    let z = delta.dot(forward);
                    vertices.push([
                        (delta.dot(right) / (half * aspect)) as f32,
                        (delta.dot(up) / half) as f32,
                        (far / (far - near) * z - far * near / (far - near)) as f32,
                        z as f32,
                    ]);
                    vertices.push([
                        instance.material.base_color[0] * shade,
                        instance.material.base_color[1] * shade,
                        instance.material.base_color[2] * shade,
                        1.,
                    ]);
                }
            }
        }
        let vertex_count = (vertices.len() / 2) as u32;
        if vertices.is_empty() {
            vertices.push([0.; 4]);
        }
        let row = (s.width * 4).div_ceil(256) * 256;
        let output_size = u64::from(row) * u64::from(s.height);
        let vertex_bytes = vertices.len() as u64 * 16;
        if vertex_bytes > self.capabilities.max_storage_binding_bytes as u64
            || vertex_bytes + output_size * 3 > s.max_bytes
            || s.width > self.device.limits().max_texture_dimension_2d
            || s.height > self.device.limits().max_texture_dimension_2d
        {
            return Err(Error::new("budget", "raster resources exceed profile"));
        }
        self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let source = render_kernel::raster_source()?;
        let module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Rust generated raster stages"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("approximate flat raster"),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vertex_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("fragment_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
                cache: None,
            });
        let buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("raster vertices"),
                contents: &bytes(&vertices),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("raster vertices"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        let size = wgpu::Extent3d {
            width: s.width,
            height: s.height,
            depth_or_array_layers: 1,
        };
        let color = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("raster color"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let depth = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("raster depth"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let color_view = color.create_view(&Default::default());
        let depth_view = depth.create_view(&Default::default());
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("raster readback"),
            size: output_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("raster preview"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &color_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: s.environment[0] as f64,
                            g: s.environment[1] as f64,
                            b: s.environment[2] as f64,
                            a: 1.,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.draw(0..vertex_count, 0..1);
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &color,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(s.height),
                },
            },
            size,
        );
        self.queue.submit([encoder.finish()]);
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
            return Err(fail("raster_execution", error));
        }
        let view = staging.slice(..).get_mapped_range();
        let mut pixels = vec![];
        for bytes in view.chunks_exact(row as usize) {
            for pixel in bytes[..s.width as usize * 4].chunks_exact(4) {
                pixels.push(pixel.try_into().expect("RGBA"));
            }
        }
        drop(view);
        staging.unmap();
        Ok(pixels)
    }
}
