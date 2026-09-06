use glam::{DVec3, Vec3, Vec4};
use render_core::{document::*, geometry::*, gltf_scene::*, render::*, *};
use serde_json::{Value, json};
const JSON: &[u8] = include_bytes!("../../../fixtures/vertex-colors/data/rgba.gltf");
const BIN: &[u8] = include_bytes!("../../../fixtures/vertex-colors/data/rgba.bin");
fn source() -> Value {
    serde_json::from_slice(JSON).unwrap()
}
fn imported(v: &Value, b: &[u8]) -> Result<Imported> {
    import_pbr_scene(
        &serde_json::to_vec(v).unwrap(),
        &[b.to_vec()],
        &[],
        Id(9950),
        &PbrPolicy {
            allow_approximations: true,
        },
    )
}
fn document(v: &Value, b: &[u8]) -> Document {
    let mut d = Document::new(Snapshot::empty(Id(9950))).unwrap();
    let commands = imported(v, b).unwrap().commands;
    d.execute(
        &fixtures::principal(),
        &fixtures::request(&d, "color:fixture:01", commands).unwrap(),
    )
    .unwrap();
    d
}
fn settings() -> Settings {
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
    s
}
fn linear(b: u8) -> f32 {
    let x = f32::from(b) / 255.;
    if x <= 0.04045 {
        x / 12.92
    } else {
        ((x + 0.055) / 1.055).powf(2.4)
    }
}
fn base() -> Vec3 {
    Vec3::new(linear(128) * 0.6, linear(192) * 0.4, 0.2)
}
fn colored(d: &Document) -> (&String, &Mesh) {
    let (id, mesh) = d
        .snapshot()
        .meshes
        .iter()
        .find(|(_, m)| m.color_attribute().is_some())
        .unwrap();
    (id, mesh)
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn linear_rgba_interpolation_shared_material_and_archive_are_exact() {
    let d = document(&source(), BIN);
    assert_eq!(d.snapshot().version, 14);
    let scene = Evaluator::default().evaluate(d.snapshot()).unwrap();
    assert_eq!(scene.instances.len(), 2);
    assert_eq!(
        scene.instances[0].material.id,
        scene.instances[1].material.id
    );
    for (x, color) in [
        (-1.75, Vec4::new(0.25, 0.75, 0.75, 0.25)),
        (0.75, Vec4::ONE),
    ] {
        let ray = Ray {
            origin: DVec3::new(x, 0.5, 3.),
            direction: -DVec3::Z,
        };
        let hit = scene.intersect(ray, 0.1, 5.).unwrap();
        assert!(hit.color_rgba(&scene).distance(color) < 1e-7);
        let value = shading(&scene, &hit, [ray; 2]);
        assert!(value.color.distance(base() * color.truncate()) < 1e-7);
        assert_eq!(value.emission, Vec3::new(0.1, 0.2, 0.3));
    }
    let before = canonical(&d).unwrap();
    let image = render(&scene, &settings(), || false).unwrap();
    let restored = storage::Envelope::new(0, None, &d)
        .unwrap()
        .decode()
        .unwrap();
    assert_eq!(canonical(&restored).unwrap(), before);
    assert_eq!(
        render(
            &Evaluator::default().evaluate(restored.snapshot()).unwrap(),
            &settings(),
            || false
        )
        .unwrap()
        .linear,
        image.linear
    );
    assert_eq!(canonical(&d).unwrap(), before);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn float_normalized_sparse_colors_agree_and_rgb_defaults_alpha_to_one() {
    let variants: [(&[u8], &[u8]); 5] = [
        (JSON, BIN),
        (
            include_bytes!("../../../fixtures/vertex-colors/data/u8.gltf"),
            include_bytes!("../../../fixtures/vertex-colors/data/u8.bin"),
        ),
        (
            include_bytes!("../../../fixtures/vertex-colors/data/u16.gltf"),
            include_bytes!("../../../fixtures/vertex-colors/data/u16.bin"),
        ),
        (
            include_bytes!("../../../fixtures/vertex-colors/data/sparse.gltf"),
            include_bytes!("../../../fixtures/vertex-colors/data/sparse.bin"),
        ),
        (
            include_bytes!("../../../fixtures/vertex-colors/data/rgb.gltf"),
            include_bytes!("../../../fixtures/vertex-colors/data/rgb.bin"),
        ),
    ];
    let mut reference = None;
    for (i, (j, b)) in variants.into_iter().enumerate() {
        let d = document(&serde_json::from_slice(j).unwrap(), b);
        let a = colored(&d).1.color_attribute().unwrap();
        assert_eq!(a.id, Id(200));
        let AttributeValues::Vec4(c) = &a.values else {
            panic!("vec4 required")
        };
        assert_eq!(
            c[0],
            if i == 4 {
                [0., 0., 1., 1.]
            } else {
                [0., 0., 1., 0.]
            }
        );
        let scene = Evaluator::default().evaluate(d.snapshot()).unwrap();
        let rgb = render(&scene, &settings(), || false).unwrap().linear;
        if let Some(ref expected) = reference {
            assert_eq!(&rgb, expected);
        } else {
            reference = Some(rgb);
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn source_colors_clamp_before_interpolation_and_reject_malformed_accessors() {
    let v = source();
    let a = v["meshes"][0]["primitives"][0]["attributes"]["COLOR_0"]
        .as_u64()
        .unwrap() as usize;
    let view = v["accessors"][a]["bufferView"].as_u64().unwrap() as usize;
    let start = v["bufferViews"][view]["byteOffset"].as_u64().unwrap() as usize;
    let mut bin = BIN.to_vec();
    bin[start..start + 4].copy_from_slice(&(-2f32).to_le_bytes());
    bin[start + 8..start + 12].copy_from_slice(&3f32.to_le_bytes());
    let result = imported(&v, &bin).unwrap();
    assert!(
        result
            .report
            .losses
            .iter()
            .any(|l| l.contains("2 components clamped"))
    );
    assert_eq!(
        colored(&document(&v, &bin)).1.color_rgba(0).unwrap(),
        Vec4::new(0., 0., 1., 0.)
    );
    bin[start..start + 4].copy_from_slice(&f32::NAN.to_le_bytes());
    assert!(imported(&v, &bin).is_err());
    for (field, value) in [
        ("type", json!("VEC2")),
        ("count", json!(3)),
        ("componentType", json!(5125)),
        ("normalized", json!(true)),
    ] {
        let mut bad = v.clone();
        bad["accessors"][a][field] = value;
        assert!(imported(&bad, BIN).is_err(), "{field}");
    }
    let mut bad = v.clone();
    bad["meshes"][0]["primitives"][0]["attributes"]["COLOR_1"] = json!(a);
    assert_eq!(imported(&bad, BIN).err().unwrap().code, "unsupported_gltf");
    let mut u: Value = serde_json::from_slice(include_bytes!(
        "../../../fixtures/vertex-colors/data/u8.gltf"
    ))
    .unwrap();
    u["accessors"][a]["normalized"] = json!(false);
    assert!(
        imported(
            &u,
            include_bytes!("../../../fixtures/vertex-colors/data/u8.bin")
        )
        .is_err()
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn vertex_alpha_multiplies_factor_and_texture_for_mask_blend_only() {
    for mode in ["MASK", "BLEND", "OPAQUE"] {
        let mut v = source();
        v["scenes"][0]["nodes"] = json!([0]);
        v["nodes"][0]["translation"] = json!([0, 0, 0]);
        v["materials"][0]["alphaMode"] = json!(mode);
        v["materials"][0]["alphaCutoff"] = json!(0.75 * 128. / 255. * 0.5);
        v["materials"][0]["emissiveFactor"] = json!([0, 1, 0]);
        v["materials"][0]["pbrMetallicRoughness"]["baseColorFactor"] = json!([0, 0, 0, 0.75]);
        let d = document(&v, BIN);
        let scene = Evaluator::default().evaluate(d.snapshot()).unwrap();
        let mut s = settings();
        s.width = 16;
        s.height = 8;
        s.samples = 256;
        s.max_depth = 1;
        s.environment = [0.; 3];
        s.light.intensity = [0.; 3];
        s.camera.lens = Some(cameras::Lens::Orthographic {
            xmag: 1.,
            ymag: 1.,
            near: 0.1,
            far: 5.,
        });
        let image = render(&scene, &s, || false).unwrap();
        for x in 0..16 {
            let mean = (0..8)
                .map(|y| f64::from(image.linear[y * 16 + x][1]))
                .sum::<f64>()
                / 8.;
            let expected = match mode {
                "MASK" => {
                    if x >= 8 {
                        1.
                    } else {
                        0.
                    }
                }
                "BLEND" => (x as f64 + 0.5) / 16. * 0.75 * 128. / 255.,
                _ => 1.,
            };
            assert!(
                (mean - expected).abs() < 0.035,
                "{mode} column{x}: {mean} vs{expected}"
            );
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn colors_survive_displacement_modeling_and_bake_without_tinting_emission() {
    let d = document(&source(), BIN);
    let mesh = colored(&d).1;
    let displacement = displacement::Displacement {
        height: displacement::Height::Constant { meters: 0.1 },
        subdivisions: 2,
        max_vertices: 1024,
    };
    let (derived, receipt) = displacement
        .evaluate(mesh, &Default::default(), || false)
        .unwrap();
    assert!(receipt.approximation.starts_with("uniform-rgba-uv-v1"));
    assert_eq!(derived.color_attribute().unwrap().id, Id(200));
    for (i, c) in derived.corners.iter().enumerate() {
        let p = derived.positions.get(c.vertex as usize);
        let u = (p.x as f32 + 1.) / 2.;
        let v = (p.y as f32 + 1.) / 2.;
        assert!(
            derived
                .color_rgba(i)
                .unwrap()
                .distance(Vec4::new(u, v, 1. - u, u))
                < 1e-7
        );
    }
    let (split, _) = modeling::apply(
        mesh,
        &modeling::Operation::SplitEdge {
            edge: mesh.edge_ids[0],
            fraction: 0.25,
        },
        &Default::default(),
        || false,
    )
    .unwrap();
    for (i, c) in split.corners.iter().enumerate() {
        let p = split.positions.get(c.vertex as usize);
        let u = (p.x as f32 + 1.) / 2.;
        let v = (p.y as f32 + 1.) / 2.;
        assert!(
            split
                .color_rgba(i)
                .unwrap()
                .distance(Vec4::new(u, v, 1. - u, u))
                < 1e-7
        );
    }
    let mut seam = mesh.clone();
    let mut a = seam.color_attribute().unwrap().clone();
    a.domain = Domain::Corner;
    a.values = AttributeValues::Vec4(
        (0..6)
            .map(|i| {
                if i < 3 {
                    [1., 0., 0., 0.]
                } else {
                    [0., 0., 1., 1.]
                }
            })
            .collect(),
    );
    seam.attributes.insert("color_0".into(), a);
    let (derived, _) = displacement
        .evaluate(&seam, &Default::default(), || false)
        .unwrap();
    let AttributeValues::Vec4(c) = &derived.color_attribute().unwrap().values else {
        panic!("rgba")
    };
    assert!(c.contains(&[1., 0., 0., 0.]) && c.contains(&[0., 0., 1., 1.]));
    assert!(
        c.iter()
            .all(|x| *x == [1., 0., 0., 0.] || *x == [0., 0., 1., 1.])
    );
    let scene = Evaluator::default().evaluate(d.snapshot()).unwrap();
    let entity = scene
        .instances
        .iter()
        .find(|i| i.geometry.color_attribute.is_some())
        .unwrap()
        .id;
    for pass in [products::BakePass::Albedo, products::BakePass::Emission] {
        let b = products::Bake {
            name: "color-bake".into(),
            entity,
            width: 8,
            height: 8,
            pass,
        };
        let image = products::bake(&scene, &b, &scene.revision, || false).unwrap();
        for y in 0..8 {
            for x in 0..8 {
                let u = (x as f32 + 0.5) / 8.;
                let v = (y as f32 + 0.5) / 8.;
                let expected = if pass == products::BakePass::Albedo {
                    base() * Vec3::new(u, v, 1. - u)
                } else {
                    Vec3::new(0.1, 0.2, 0.3)
                };
                assert!(Vec3::from_array(image.rgb[y * 8 + x]).distance(expected) < 1e-7);
            }
        }
        assert_eq!(
            products::bake(&scene, &b, "stale", || false)
                .unwrap_err()
                .code,
            "stale_revision"
        );
        assert_eq!(
            products::bake(&scene, &b, &scene.revision, || true)
                .unwrap_err()
                .code,
            "cancelled"
        );
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn native_color_validation_versions_transactions_and_profile_boundaries() {
    let mut d = document(&source(), BIN);
    let (_, mesh) = colored(&d);
    let mesh = mesh.clone();
    let before = canonical(&d).unwrap();
    for variant in 0..5 {
        let mut m = mesh.clone();
        let a = m.attributes.get_mut("color_0").unwrap();
        match variant {
            0 => a.domain = Domain::Face,
            1 => a.values = AttributeValues::Vec3(vec![[1.; 3]; 4]),
            2 => a.values = AttributeValues::Vec4(vec![[2.; 4]; 4]),
            3 => a.values = AttributeValues::Vec4(vec![[1.; 4]; 3]),
            _ => {
                let mut duplicate = a.clone();
                duplicate.id = Id(201);
                m.attributes.insert("duplicate".into(), duplicate);
            }
        }
        let q = fixtures::request(
            &d,
            &format!("color:negative:{variant:02}"),
            vec![Command::PutMesh { mesh: m }],
        )
        .unwrap();
        assert!(d.execute(&fixtures::principal(), &q).is_err());
        assert_eq!(canonical(&d).unwrap(), before);
    }
    let mut old = d.snapshot().clone();
    old.version = 13;
    assert!(old.validate().is_err());
    let mut q = fixtures::request(
        &d,
        "color:stale:00001",
        vec![Command::PutMesh { mesh: mesh.clone() }],
    )
    .unwrap();
    q.base_revision = "0".repeat(64);
    assert_eq!(
        d.execute(&fixtures::principal(), &q).unwrap_err().code,
        "stale_revision"
    );
    assert_eq!(
        Evaluator::default()
            .evaluate_with_cancel(d.snapshot(), || true)
            .unwrap_err()
            .code,
        "cancelled"
    );
    let mut scene = Evaluator::default().evaluate(d.snapshot()).unwrap();
    assert_eq!(
        render(&scene, &settings(), || true).unwrap_err().code,
        "cancelled"
    );
    let mut s = settings();
    s.max_bytes = 1;
    assert_eq!(render(&scene, &s, || false).unwrap_err().code, "budget");
    for i in &mut scene.instances {
        i.material.pbr = None;
    }
    assert_eq!(
        render(&scene, &settings(), || false).unwrap_err().code,
        "unsupported_profile"
    );
    for report in [
        interchange::export_obj(&mesh).unwrap().1,
        interchange::export_gltf(&mesh).unwrap().2,
    ] {
        assert!(
            report
                .losses
                .iter()
                .any(|l| l.contains("linear RGBA color attribute")
                    && l.contains("000000000000000000000000000000c8"))
        );
    }
    assert_eq!(canonical(&d).unwrap(), before);
}
