use render_core::{document::*, gltf_scene::*, render::*, *};
use std::sync::atomic::AtomicBool;
#[test]
#[ignore = "requires a real GPU; mandatory in named UV acceptance"]
fn named_uv_cpu_gpu_roles_mips_and_displacement_match() {
    let mut d = Document::new(Snapshot::empty(Id(9900))).unwrap();
    let imported = import_pbr_glb(
        include_bytes!("../../../fixtures/named-uv/data/roles.glb"),
        Id(9900),
        &PbrPolicy {
            allow_approximations: true,
        },
    )
    .unwrap();
    d.execute(
        &fixtures::principal(),
        &fixtures::request(&d, "uv:gpu:fixture:01", imported.commands).unwrap(),
    )
    .unwrap();
    let mut s = fixtures::settings();
    s.width = 24;
    s.height = 24;
    s.samples = 8;
    s.max_depth = 2;
    s.camera.position = [0., 0., 3.];
    s.camera.target = [0.; 3];
    s.camera.lens = Some(cameras::Lens::Orthographic {
        xmag: 1.,
        ymag: 1.,
        near: 0.1,
        far: 5.,
    });
    let mut gpu = pollster::block_on(render_gpu::Renderer::new()).unwrap();
    let cancelled = AtomicBool::new(false);
    for variant in 0..3 {
        let mut snapshot = d.snapshot().clone();
        if variant == 2 {
            let p = snapshot
                .materials
                .values_mut()
                .find_map(|m| m.pbr.as_mut().filter(|p| p.base_color.is_some()))
                .unwrap();
            let mut binding = p.base_color.clone().unwrap();
            binding.uv_attribute = Some(Id(101));
            binding.role = textures::TextureRole::LinearData;
            p.displacement = Some(displacement::Displacement {
                height: displacement::Height::Image {
                    binding,
                    scale_meters: 0.1,
                    bias_meters: 0.,
                },
                subdivisions: 2,
                max_vertices: 256,
            });
        }
        let mut scene = Evaluator::default().evaluate(&snapshot).unwrap();
        if variant == 1 {
            let inst = &mut scene.instances[0];
            inst.material
                .pbr
                .as_mut()
                .unwrap()
                .emission
                .as_mut()
                .unwrap()
                .sampler
                .min = textures::MinFilter::LinearMipLinear;
            let slot = inst
                .geometry
                .uv_attributes
                .iter()
                .position(|id| *id == Id(101))
                .unwrap();
            for t in &mut std::sync::Arc::make_mut(&mut inst.geometry).triangles {
                for uv in &mut t.uv_sets[slot] {
                    *uv *= 32.;
                }
            }
            inst.geometry_id = digest(
                &canonical(&(inst.geometry_id.clone(), "32x selected UV footprint")).unwrap(),
            );
        }
        let cpu = render(&scene, &s, || false).unwrap();
        let actual = pollster::block_on(gpu.render(&scene, &s, 0, &cancelled)).unwrap();
        let rmse = (cpu
            .linear
            .iter()
            .flatten()
            .zip(actual.linear.iter().flatten())
            .map(|(a, b)| f64::from(a - b).powi(2))
            .sum::<f64>()
            / (s.width * s.height * 3) as f64)
            .sqrt();
        assert!(rmse < 0.002, "variant {variant}: {rmse}");
        assert_eq!(cpu.objects, actual.objects);
        assert!(
            cpu.normals
                .iter()
                .flatten()
                .zip(actual.normals.iter().flatten())
                .all(|(a, b)| (a - b).abs() < 0.002)
        );
        assert!(
            cpu.depth
                .iter()
                .zip(actual.depth.iter())
                .all(|(a, b)| (a - b).abs() < 2e-5)
        );
    }
}
