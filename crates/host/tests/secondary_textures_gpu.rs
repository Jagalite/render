use render_core::{document::*, render::*, textures::*, *};
use std::sync::atomic::AtomicBool;
#[test]
#[ignore = "requires actual GPU; mandatory in secondary footprint acceptance"]
fn secondary_emitter_filters_match_cpu_and_reuse_both_gpu_profiles() {
    let mut gpu = pollster::block_on(render_gpu::Renderer::new()).unwrap();
    let cancel = AtomicBool::new(false);
    let mut d = Document::new(Snapshot::empty(Id(10070))).unwrap();
    let imported = gltf_scene::import_pbr_glb(
        include_bytes!("../../../fixtures/secondary-textures/data/mask-nearest.glb"),
        Id(10070),
        &gltf_scene::PbrPolicy {
            allow_approximations: true,
        },
    )
    .unwrap();
    d.execute(
        &fixtures::principal(),
        &fixtures::request(&d, "footprint:gpu:import", imported.commands).unwrap(),
    )
    .unwrap();
    let mut s = fixtures::settings();
    s.width = 8;
    s.height = 8;
    s.samples = 32;
    s.max_depth = 2;
    s.environment = [0.; 3];
    s.light.intensity = [0.; 3];
    s.camera.position = [0., 0., 1.];
    s.camera.target = [0.; 3];
    s.camera.lens = Some(cameras::Lens::Orthographic {
        xmag: 0.9,
        ymag: 0.9,
        near: 0.01,
        far: 10.,
    });
    let mut opaque = None;
    for extended in [false, true, false] {
        let mut baseline = None;
        for min in [MinFilter::Nearest, MinFilter::LinearMipLinear] {
            let mut snapshot = d.snapshot().clone();
            for m in snapshot.materials.values_mut() {
                if let Some(p) = m.pbr.as_mut() {
                    if p.advanced.is_some() && !extended {
                        p.advanced = None;
                    }
                    if let Some(b) = p.emission.as_mut() {
                        b.sampler.min = min;
                    }
                }
            }
            let scene = Evaluator::default().evaluate(&snapshot).unwrap();
            let cpu = render(&scene, &s, || false).unwrap();
            let image = pollster::block_on(gpu.render(&scene, &s, 0, &cancel)).unwrap();
            let rmse = (cpu
                .linear
                .iter()
                .flatten()
                .zip(image.linear.iter().flatten())
                .map(|(a, b)| f64::from(a - b).powi(2))
                .sum::<f64>()
                / (s.width * s.height * 3) as f64)
                .sqrt();
            println!("extended={extended} filter={min:?} cpu_gpu_rmse={rmse}");
            assert!(rmse < 0.002);
            assert_eq!(cpu.objects, image.objects);
            assert_eq!(cpu.depth, image.depth);
            if let Some(ref baseline) = baseline {
                assert_eq!(&image.linear, baseline);
            } else {
                baseline = Some(image.linear.clone());
            }
            if !extended {
                if let Some(ref previous) = opaque {
                    assert_eq!(previous, &image.linear);
                } else {
                    opaque = Some(image.linear);
                }
            }
        }
    }
    assert_eq!(
        render_core::digest(render_kernel::path::kernel().generate().unwrap().as_bytes()),
        "sha256:e0bae9a561867ddc4622b68e9d52635ce3d380bbf6f4aeb109e63b46c8d89247"
    );
    assert_eq!(
        render_core::digest(
            render_kernel::path::alpha_kernel()
                .generate()
                .unwrap()
                .as_bytes()
        ),
        "sha256:82177b8262a499bad4e9997fe15d04975b9635e0817ea4caeeb38d713a0648dc"
    );
}
