use render_core::{document::*, gltf_scene::*, render::*, *};
use std::sync::atomic::AtomicBool;
#[test]
#[ignore = "requires a real GPU; mandatory in vertex-color acceptance"]
fn vertex_color_cpu_gpu_encodings_displacement_and_admission_match() {
    let mut gpu = pollster::block_on(render_gpu::Renderer::new()).unwrap();
    let variants: [&[u8]; 5] = [
        include_bytes!("../../../fixtures/vertex-colors/data/rgba.glb"),
        include_bytes!("../../../fixtures/vertex-colors/data/rgb.glb"),
        include_bytes!("../../../fixtures/vertex-colors/data/u8.glb"),
        include_bytes!("../../../fixtures/vertex-colors/data/u16.glb"),
        include_bytes!("../../../fixtures/vertex-colors/data/sparse.glb"),
    ];
    let mut s = fixtures::settings();
    s.width = 32;
    s.height = 16;
    s.samples = 8;
    s.max_depth = 2;
    s.camera.position = [0., 0., 3.];
    s.camera.target = [0.; 3];
    s.camera.lens = Some(cameras::Lens::Orthographic {
        xmag: 2.5,
        ymag: 1.,
        near: 0.1,
        far: 5.,
    });
    let cancel = AtomicBool::new(false);
    let mut reference = None;
    for (variant, bytes) in variants.into_iter().enumerate() {
        let mut d = Document::new(Snapshot::empty(Id(9950))).unwrap();
        let imported = import_pbr_glb(
            bytes,
            Id(9950),
            &PbrPolicy {
                allow_approximations: true,
            },
        )
        .unwrap();
        d.execute(
            &fixtures::principal(),
            &fixtures::request(&d, "color:gpu:fixture:01", imported.commands).unwrap(),
        )
        .unwrap();
        for displaced in [false, true] {
            let mut snapshot = d.snapshot().clone();
            if displaced {
                for m in snapshot.materials.values_mut() {
                    if let Some(p) = m.pbr.as_mut() {
                        p.displacement = Some(displacement::Displacement {
                            height: displacement::Height::Constant { meters: 0.1 },
                            subdivisions: 2,
                            max_vertices: 256,
                        });
                    }
                }
            }
            let mut scene = Evaluator::default().evaluate(&snapshot).unwrap();
            let cpu = render(&scene, &s, || false).unwrap();
            let actual = pollster::block_on(gpu.render(&scene, &s, 0, &cancel)).unwrap();
            let rmse = (cpu
                .linear
                .iter()
                .flatten()
                .zip(actual.linear.iter().flatten())
                .map(|(a, b)| f64::from(a - b).powi(2))
                .sum::<f64>()
                / (s.width * s.height * 3) as f64)
                .sqrt();
            assert!(
                rmse < 0.002,
                "variant{variant} displaced{displaced}: {rmse}"
            );
            assert_eq!(cpu.objects, actual.objects);
            assert!(
                cpu.depth
                    .iter()
                    .zip(&actual.depth)
                    .all(|(a, b)| (a - b).abs() < 2e-5)
            );
            assert!(
                cpu.normals
                    .iter()
                    .flatten()
                    .zip(actual.normals.iter().flatten())
                    .all(|(a, b)| (a - b).abs() < 2e-5)
            );
            println!("variant={variant} displaced={displaced} linear_rmse={rmse}");
            if !displaced {
                if let Some(ref rgb) = reference {
                    assert_eq!(&actual.linear, rgb);
                } else {
                    reference = Some(actual.linear);
                }
            }
            assert_eq!(
                pollster::block_on(gpu.render(&scene, &s, 0, &AtomicBool::new(true)))
                    .unwrap_err()
                    .code,
                "cancelled"
            );
            let mut small = s.clone();
            small.max_bytes = 1;
            assert_eq!(
                pollster::block_on(gpu.render(&scene, &small, 0, &cancel))
                    .unwrap_err()
                    .code,
                "budget"
            );
            assert_eq!(
                pollster::block_on(gpu.raster_preview(&scene, &s))
                    .unwrap_err()
                    .code,
                "unsupported_profile"
            );
            for i in &mut scene.instances {
                i.material.pbr = None;
            }
            assert_eq!(
                pollster::block_on(gpu.render(&scene, &s, 0, &cancel))
                    .unwrap_err()
                    .code,
                "unsupported_profile"
            );
            assert_eq!(
                pollster::block_on(gpu.raster_preview(&scene, &s))
                    .unwrap_err()
                    .code,
                "unsupported_profile"
            );
        }
    }
}
