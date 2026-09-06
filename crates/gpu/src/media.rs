//! Private bounded packing of typed evaluated RGB media.
use super::*;
use glam::{DAffine3, DVec3};
pub const MAX_CELLS: usize = 64;
pub const MAX_CELL_VISITS: u64 = 8_388_608;
pub const MAX_CORNER_ERROR: f64 = 1e-5;
const MAX_COORDINATE: f64 = 1e12;
const MAX_INVERSE_COMPONENT: f64 = 1e12;
pub(super) struct Cells {
    pub rows: Vec<[f32; 4]>,
    pub count: usize,
    pub max_corner_error: f64,
}
pub(super) fn pack(scene: &Scene, settings: &Settings, camera: DVec3) -> Result<Option<Cells>> {
    if scene.media.is_empty() {
        return Ok(None);
    }
    let count = scene.media.cell_count();
    if scene
        .instances
        .iter()
        .any(|i| i.material.pbr.as_ref().is_none_or(|p| p.advanced.is_some()))
        || scene
            .media
            .transport_cells()
            .iter()
            .any(|c| c.scattering != DVec3::ZERO)
    {
        return Err(Error::new(
            "unsupported_profile",
            "GPU media requires zero evaluated scattering and ordinary opaque PBR surfaces",
        ));
    }
    if count > MAX_CELLS {
        return Err(Error::new("budget", "GPU media exceeds 64 occupied cells"));
    }
    let n = count as u64;
    let visits = [
        u64::from(settings.width),
        u64::from(settings.height),
        u64::from(settings.samples),
        u64::from(settings.max_depth),
        n,
        2 * n + 4,
    ]
    .into_iter()
    .try_fold(1u64, |a, b| a.checked_mul(b));
    if visits.is_none_or(|v| v > MAX_CELL_VISITS) {
        return Err(Error::new(
            "budget",
            "GPU media exceeds 8388608 bounded cell visits per dispatch",
        ));
    }
    let mut out = Cells {
        rows: Vec::with_capacity(count * 7),
        count,
        max_corner_error: 0.,
    };
    for cell in scene.media.transport_cells() {
        // Unit-cell coordinates preserve affine metric ray parameters without
        // subtracting large local bound values in the generated f32 traversal.
        let unit = DAffine3::from_scale((cell.bounds.max - cell.bounds.min).recip())
            * DAffine3::from_translation(-cell.bounds.min)
            * cell.inverse
            * DAffine3::from_translation(camera);
        // Bound arithmetic away from the shader's 1e30 escape sentinel and
        // reject subnormal coefficients that a device may flush to zero.
        if cell
            .extinction
            .to_array()
            .into_iter()
            .chain(cell.emission.to_array())
            .any(|x| x != 0. && (!(x as f32).is_normal() || !(x as f32).is_finite()))
            || unit.to_cols_array().iter().any(|x| {
                !x.is_finite()
                    || x.abs() > MAX_INVERSE_COMPONENT
                    || (*x != 0. && !(*x as f32).is_normal())
            })
        {
            return Err(Error::new(
                "precision",
                "GPU medium coefficients and inverse components must be normal f32 or zero, with inverse magnitude at most 1e12",
            ));
        }
        let quantized = unit.to_cols_array().map(|x| f64::from(x as f32));
        let packed = DAffine3::from_cols_array(&quantized);
        if !packed.is_finite() || packed.matrix3.determinant() == 0. {
            return Err(Error::new(
                "precision",
                "GPU medium inverse unit box is nonfinite or singular in f32",
            ));
        }
        let reference = unit.inverse();
        // Keep translated bounds separate from the ray origin. Adding a tiny
        // camera jitter to a per-cell translation can round a shared face
        // outside BOTH neighboring half-open cells.
        let lower = (-unit.translation).as_vec3().as_dvec3();
        let upper = (DVec3::ONE - unit.translation).as_vec3().as_dvec3();
        if !lower.is_finite() || !upper.is_finite() || !lower.cmplt(upper).all() {
            return Err(Error::new(
                "precision",
                "GPU medium shifted bounds collapse in f32",
            ));
        }
        let reconstructed = packed.matrix3.inverse();
        for mask in 0..8 {
            let corner = DVec3::from_array(std::array::from_fn(|axis| {
                f64::from(mask & (1 << axis) != 0)
            }));
            let original = reference.transform_point3(corner);
            let bound = DVec3::from_array(std::array::from_fn(|axis| {
                if mask & (1 << axis) == 0 {
                    lower[axis]
                } else {
                    upper[axis]
                }
            }));
            let restored = reconstructed * bound;
            if !original.is_finite()
                || !restored.is_finite()
                || original.abs().max_element() > MAX_COORDINATE
                || restored.abs().max_element() > MAX_COORDINATE
            {
                return Err(Error::new(
                    "precision",
                    "GPU medium camera-relative corners exceed 1e12 world meters",
                ));
            }
            let error = original.distance(restored);
            if !error.is_finite() || error > MAX_CORNER_ERROR {
                return Err(Error::new(
                    "precision",
                    "GPU medium unit-box corner error exceeds 1e-5 world meters",
                ));
            }
            out.max_corner_error = out.max_corner_error.max(error);
        }
        for column in quantized[..9].chunks_exact(3) {
            out.rows
                .push([column[0] as f32, column[1] as f32, column[2] as f32, 0.]);
        }
        out.rows.push(point(lower, 0.));
        out.rows.push(point(upper, 0.));
        out.rows.push(point(cell.extinction, 0.));
        out.rows.push(point(cell.emission, 0.));
    }
    Ok(Some(out))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use render_core::{Id, volumes};
    use render_kernel::{Expr, Stmt, Ty, call, f, if_, let_, read, set, u};
    use std::{
        future::Future,
        sync::Arc,
        task::{Context, Poll, Wake, Waker},
    };
    fn block_on<T>(future: impl Future<Output = T>) -> T {
        struct Parker(std::thread::Thread);
        impl Wake for Parker {
            fn wake(self: Arc<Self>) {
                self.0.unpark();
            }
        }
        let waker = Waker::from(Arc::new(Parker(std::thread::current())));
        let mut context = Context::from_waker(&waker);
        let mut future = Box::pin(future);
        loop {
            match future.as_mut().poll(&mut context) {
                Poll::Ready(value) => return value,
                Poll::Pending => std::thread::park(),
            }
        }
    }
    #[test]
    #[ignore = "requires real GPU; exact half-open media-ray contract"]
    fn half_open_unit_cells_and_sparse_holes_execute_on_device() {
        let renderer = block_on(Renderer::new()).unwrap();
        let asset = volumes::Asset {
            origin: [0.; 3],
            voxel_size: [1.; 3],
            cells: vec![
                volumes::Cell {
                    coordinate: [0, 0, 0],
                    density: 1.,
                    emission: [0.2, 0.4, 0.6],
                },
                volumes::Cell {
                    coordinate: [1, 0, 0],
                    density: 3.,
                    emission: [0.; 3],
                },
                volumes::Cell {
                    coordinate: [0, 0, 2],
                    density: 2.,
                    emission: [0.; 3],
                },
            ],
            absorption: [0.5, 1., 2.],
            scattering: [0.; 3],
            anisotropy: 0.,
            max_step_meters: 0.1,
        };
        let mut scene = render_core::render::Evaluator::default()
            .evaluate(&render_core::document::Snapshot::empty(Id(1)))
            .unwrap();
        scene.media =
            volumes::Media::build([(Id(1), &asset, DAffine3::IDENTITY)], || false).unwrap();
        let mut settings = render_core::fixtures::settings();
        settings.width = 1;
        settings.height = 1;
        settings.samples = 1;
        settings.max_depth = 1;
        settings.camera.position = [1., 0., 0.];
        settings.camera.target = [1., 0., 1.];
        let mut packed = super::super::pack(&scene, &settings, 0).unwrap();
        // Each ray is exact in f32. Camera sampling is deliberately absent here.
        let rays: [([f64; 3], [f64; 3]); 8] = [
            ([0., 0.5, -1.], [0., 0., 1.]),
            ([1., 0.5, -1.], [0., 0., 1.]),
            ([2., 0.5, -1.], [0., 0., 1.]),
            ([0.5, 1., -1.], [0., 0., 1.]),
            ([0.5, 0.5, -1.], [0., 0., 1.]),
            ([0.5, 0.5, 4.], [0., 0., -1.]),
            ([1. - 2f64.powi(-26), 0.5, -1.], [0., 0., 1.]),
            ([1. + 2f64.powi(-26), 0.5, -1.], [0., 0., 1.]),
        ];
        for (origin, direction) in rays {
            packed.params.push([
                (origin[0] - 1.) as f32,
                origin[1] as f32,
                origin[2] as f32,
                0.,
            ]);
            packed.params.push([
                direction[0] as f32,
                direction[1] as f32,
                direction[2] as f32,
                10.,
            ]);
        }
        let index = Expr::var("index", Ty::U32);
        let a = Expr::var("ray_origin", Ty::V4);
        let b = Expr::var("ray_direction", Ty::V4);
        let mut kernel = render_kernel::path::media_kernel();
        kernel.body = vec![
            let_("index", Expr::var("gid", Ty::U3).field("x")),
            if_(
                index.clone().ge(u(rays.len() as u32)),
                vec![Stmt::Return(None)],
            ),
            let_("ray_origin", read("params", u(12) + index.clone() * u(2))),
            let_(
                "ray_direction",
                read("params", u(13) + index.clone() * u(2)),
            ),
        ];
        for (offset, name) in [(0, "medium_transmittance"), (1, "medium_radiance")] {
            let value = call(
                name,
                Ty::V3,
                vec![
                    a.clone().field("xyz"),
                    b.clone().field("xyz"),
                    a.clone().field("w"),
                    b.clone().field("w"),
                ],
            );
            kernel.body.push(set(
                read("output", index.clone() * u(2) + u(offset)),
                call("vec4<f32>", Ty::V4, vec![value, f(0.)]),
            ));
        }
        let device = &renderer.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Rust medium ray contract"),
            source: wgpu::ShaderSource::Wgsl(kernel.generate().unwrap().into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("exact medium rays"),
            layout: None,
            module: &module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let geometry = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: &bytes(&packed.geometry),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: &bytes(&packed.params),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let size = rays.len() as u64 * 32;
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let entries = [(0, &geometry), (2, &params), (3, &output)].map(|(binding, buffer)| {
            wgpu::BindGroupEntry {
                binding,
                resource: buffer.as_entire_binding(),
            }
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&output, 0, &staging, 0, size);
        renderer.queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        staging
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
        device.poll(wgpu::PollType::Wait).unwrap();
        rx.recv().unwrap().unwrap();
        let view = staging.slice(..).get_mapped_range();
        let values: Vec<f32> = view
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
            .collect();
        for (index, (origin, direction)) in rays.into_iter().enumerate() {
            let ray = render_core::render::Ray {
                origin: DVec3::from_array(origin),
                direction: DVec3::from_array(direction),
            };
            let reference = scene
                .media
                .transport(ray, 0., 10., |_, _| panic!("no scattering"), || false)
                .unwrap();
            let expected_tau = if index == 2 || index == 3 { 0. } else { 3. };
            for c in 0..3 {
                let t = (-asset.absorption[c] * expected_tau).exp();
                assert!(
                    (f64::from(values[index * 8 + c]) - t).abs() < 2e-6,
                    "ray{index} transmittance"
                );
                assert!(
                    (f64::from(values[index * 8 + 4 + c]) - reference.radiance[c]).abs() < 2e-6,
                    "ray{index} emission"
                );
            }
        }
        drop(view);
        staging.unmap();
    }
}
