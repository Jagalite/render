use glam::DVec3;
use render_core::{document::*, geometry::*, render::*, *};

// Parallel plates approximate infinite planes. At the bounded deterministic
// sample coordinates used here every path hits; there is no environment light.
fn plates(pbr_bottom: bool, ceiling_color: [f32; 3]) -> (Document, Settings) {
    let mut doc = Document::new(Snapshot::empty(Id(9100))).unwrap();
    let mut commands = vec![];
    for (i, z) in [0., 2.].into_iter().enumerate() {
        let mesh = Mesh::from_polygons(
            Positions::F64(vec![
                [-1e6, -1e6, z],
                [1e6, -1e6, z],
                [1e6, 1e6, z],
                [-1e6, 1e6, z],
            ]),
            &[vec![0, 1, 2, 3]],
            &[],
        )
        .unwrap();
        let key = mesh.content_id().unwrap();
        let mut material = Material::diffuse(
            Id(9200 + i as u128),
            if i == 0 {
                [0.5, 0.25, 0.75]
            } else {
                ceiling_color
            },
        );
        if i == 0 && pbr_bottom {
            material.base_color = [1.; 3];
            material.metallic = 1.;
            material.roughness = 1.;
            material.pbr = Some(pbr::Surface {
                double_sided: true,
                ..Default::default()
            });
        }
        if i == 1 {
            material.emission = [1., 0.5, 0.25];
        }
        commands.extend([
            Command::PutMesh { mesh },
            Command::PutMaterial { material },
            Command::CreateEntity {
                entity: Entity {
                    id: Id(9300 + i as u128),
                    name: format!("plate {i}"),
                    parent: None,
                    mesh: Some(key),
                    material: Some(Id(9200 + i as u128)),
                    transform: Transform::default(),
                },
            },
        ]);
    }
    let mut s = fixtures::settings();
    s.width = 1;
    s.height = 1;
    s.samples = 32768;
    s.max_depth = 2;
    s.environment = [0.; 3];
    s.light.intensity = [0.; 3];
    s.camera = Camera {
        position: [0., 0., 1.],
        target: [0., 0., 0.],
        up: [0., 1., 0.],
        vertical_fov_radians: 0.5,
        lens: Some(cameras::Lens::Orthographic {
            xmag: 0.1,
            ymag: 0.1,
            near: 0.01,
            far: 1.5,
        }),
    };
    commands.push(Command::SetRenderSettings {
        settings: s.clone(),
    });
    doc.execute(
        &fixtures::principal(),
        &fixtures::request(&doc, "plates:author:01", commands).unwrap(),
    )
    .unwrap();
    (doc, s)
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn rough_metal_indirect_emitter_matches_closed_form() {
    let (doc, mut s) = plates(true, [0.; 3]);
    let before = canonical(&doc).unwrap();
    let scene = Evaluator::default().evaluate(doc.snapshot()).unwrap();
    s.max_depth = 1;
    let direct = render(&scene, &s, || false).unwrap();
    assert_eq!(direct.linear[0], [0.; 3]);
    s.max_depth = 2;
    let indirect = render(&scene, &s, || false).unwrap();
    // At normal incidence, white metal and alpha=1: f=1/(2*pi*(1+mu)).
    // Integral of f*mu over the hemisphere is 1-ln(2).
    for (actual, emission) in indirect.linear[0].into_iter().zip([1., 0.5, 0.25]) {
        assert!(
            (f64::from(actual) - (1. - 2_f64.ln()) * emission).abs() < 0.004,
            "{actual}"
        );
    }
    assert_eq!(direct.objects, indirect.objects);
    assert_eq!(direct.depth, indirect.depth);
    assert_eq!(direct.normals, indirect.normals);
    assert!((indirect.depth[0] - 1.).abs() < 1e-6);
    assert_eq!(indirect.objects[0], Some(Id(9300)));
    assert!(indirect.receipt.backend.contains("pbr-path-v1"));
    s.max_depth = 3;
    assert_eq!(
        indirect.linear,
        render(&scene, &s, || false).unwrap().linear
    );
    assert_eq!(canonical(&doc).unwrap(), before);
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn diffuse_cavity_matches_finite_geometric_series_and_mixed_paths_continue() {
    let (doc, mut s) = plates(false, [0.5; 3]);
    s.samples = 64;
    let scene = Evaluator::default().evaluate(doc.snapshot()).unwrap();
    for depth in [1, 2, 3, 4, 6, 16] {
        s.max_depth = depth;
        let image = render(&scene, &s, || false).unwrap();
        for ((actual, a), emission) in image.linear[0]
            .into_iter()
            .zip([0.5_f64, 0.25, 0.75])
            .zip([1., 0.5, 0.25])
        {
            let expected = (0..depth / 2)
                .map(|k| a * emission * (a * 0.5).powi(k as i32))
                .sum::<f64>();
            assert!(
                (f64::from(actual) - expected).abs() < 2e-6,
                "depth {depth}: {actual} != {expected}"
            );
        }
    }
    let (doc, mut s) = plates(true, [0.5; 3]);
    s.samples = 256;
    let scene = Evaluator::default().evaluate(doc.snapshot()).unwrap();
    let two = render(&scene, &s, || false).unwrap();
    s.max_depth = 4;
    let four = render(&scene, &s, || false).unwrap();
    assert!(four.linear[0][0] > two.linear[0][0] + 0.001);
    assert_eq!(
        four.receipt.output_digest,
        render(&scene, &s, || false).unwrap().receipt.output_digest
    );
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn escape_is_counted_once_and_multibounce_operations_reject_invalid_cancelled_stale() {
    let (mut doc, mut s) = plates(true, [0.; 3]);
    s.samples = 64;
    let scene = Evaluator::default().evaluate(doc.snapshot()).unwrap();
    let mut polls = 0;
    assert_eq!(
        render(&scene, &s, || {
            polls += 1;
            polls > 4
        })
        .unwrap_err()
        .code,
        "cancelled"
    );
    for depth in [0, 17] {
        s.max_depth = depth;
        assert_eq!(
            render(&scene, &s, || false).unwrap_err().code,
            "render_settings"
        );
    }
    s.max_depth = 16;
    s.max_bytes = 1;
    assert_eq!(render(&scene, &s, || false).unwrap_err().code, "budget");
    let old = doc.snapshot().revision().unwrap();
    let mut settings = doc.snapshot().render_settings.clone().unwrap();
    settings.samples = 8;
    doc.execute(
        &fixtures::principal(),
        &fixtures::request(
            &doc,
            "plates:settings:02",
            vec![Command::SetRenderSettings { settings }],
        )
        .unwrap(),
    )
    .unwrap();
    let mut session = agent::Session::new(doc);
    let result = session.dispatch(
        &fixtures::principal(),
        agent::Request {
            version: 0,
            operation: agent::Operation::Branch {
                branch: "stale".into(),
                base_revision: old,
            },
        },
    );
    assert_eq!(result.unwrap_err().code, "conflict");
    let (mut scene, mut s, _) = fixtures::diffuse_plane().unwrap();
    scene.instances[0].material.pbr = Some(pbr::Surface {
        double_sided: true,
        ..Default::default()
    });
    scene.instances[0].material.metallic = 1.;
    scene.instances[0].material.base_color = [1.; 3];
    s.light.intensity = [0.; 3];
    s.environment = [1.; 3];
    s.samples = 64;
    s.max_depth = 1;
    let one = render(&scene, &s, || false).unwrap();
    s.max_depth = 16;
    let many = render(&scene, &s, || false).unwrap();
    for (a, b) in one
        .linear
        .iter()
        .flatten()
        .zip(many.linear.iter().flatten())
    {
        assert!((a - b).abs() < 1e-6);
    }
    assert!(DVec3::from_array(one.linear[0].map(f64::from)).is_finite());
}
