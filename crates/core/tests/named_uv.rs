use glam::{DVec3, Vec2, Vec3};
use render_core::{document::*, gltf_scene::*, render::*, textures::*, *};
use serde_json::{Value, json};
const JSON: &[u8] = include_bytes!("../../../fixtures/named-uv/data/roles.gltf");
const BIN: &[u8] = include_bytes!("../../../fixtures/named-uv/data/roles.bin");
fn source() -> Value {
    serde_json::from_slice(JSON).unwrap()
}
fn imported(v: &Value) -> Result<Imported> {
    import_pbr_scene(
        &serde_json::to_vec(v).unwrap(),
        &[BIN.to_vec()],
        &[],
        Id(9900),
        &PbrPolicy {
            allow_approximations: true,
        },
    )
}
fn settings() -> Settings {
    let mut s = fixtures::settings();
    s.width = 16;
    s.height = 16;
    s.samples = 4;
    s.max_depth = 2;
    s.camera.position = [0., 0., 3.];
    s.camera.target = [0.; 3];
    s.camera.lens = Some(cameras::Lens::Orthographic {
        xmag: 1.,
        ymag: 1.,
        near: 0.1,
        far: 5.,
    });
    s
}
fn document(v: &Value) -> Document {
    let mut d = Document::new(Snapshot::empty(Id(9900))).unwrap();
    let mut commands = imported(v).unwrap().commands;
    commands.push(Command::SetRenderSettings {
        settings: settings(),
    });
    d.execute(
        &fixtures::principal(),
        &fixtures::request(&d, "named-uv:fixture:01", commands).unwrap(),
    )
    .unwrap();
    d
}
fn linear(byte: u8) -> f32 {
    let x = f32::from(byte) / 255.;
    if x <= 0.04045 {
        x / 12.92
    } else {
        ((x + 0.055) / 1.055).powf(2.4)
    }
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn named_uv_material_roles_and_normal_frame_match_independent_samples() {
    let d = document(&source());
    assert_eq!(d.snapshot().version, 12);
    let scene = Evaluator::default().evaluate(d.snapshot()).unwrap();
    let ray = Ray {
        origin: DVec3::new(-0.5, 0.5, 3.),
        direction: -DVec3::Z,
    };
    let hit = scene.intersect(ray, 0.1, 5.).unwrap();
    assert_eq!(hit.uv, Vec2::new(0.25, 0.75));
    assert_eq!(hit.uv_for(&scene, Some(Id(101))), Vec2::new(0.75, 0.25));
    assert_eq!(hit.uv_for(&scene, Some(Id(102))), Vec2::new(0.75, 0.75));
    let s = shading(&scene, &hit, [ray; 2]);
    assert!(
        s.color
            .distance(Vec3::from_array([linear(30), linear(20), 1.]) * 0.6)
            < 1e-6
    );
    assert!(
        s.emission
            .distance(Vec3::from_array([linear(20), 1., linear(30)]) * 0.4)
            < 1e-6
    );
    assert!((s.metallic - 0.5 * 128. / 255.).abs() < 1e-6);
    assert!((s.roughness - 0.8 * 64. / 255.).abs() < 1e-6);
    assert_eq!(s.occlusion, 1.);
    let mapped = DVec3::new(
        128. / 255. * 2. - 1.,
        204. / 255. * 2. - 1.,
        230. / 255. * 2. - 1.,
    )
    .normalize();
    assert!(
        s.normal.distance(mapped) < 1e-6,
        "{:?} vs {mapped:?}",
        s.normal
    );
    let before = canonical(&d).unwrap();
    let image = render(&scene, &settings(), || false).unwrap();
    let restored = storage::Envelope::new(0, None, &d)
        .unwrap()
        .decode()
        .unwrap();
    assert_eq!(canonical(&restored).unwrap(), before);
    let restored_scene = Evaluator::default().evaluate(restored.snapshot()).unwrap();
    assert_eq!(
        render(&restored_scene, &settings(), || false)
            .unwrap()
            .linear,
        image.linear
    );
    assert_eq!(canonical(&d).unwrap(), before);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn named_uv_displacement_preserves_coordinates_and_ids() {
    let mut d = document(&source());
    let mut material = d
        .snapshot()
        .materials
        .values()
        .find(|m| m.pbr.as_ref().is_some_and(|p| p.base_color.is_some()))
        .unwrap()
        .clone();
    let mut binding = material.pbr.as_ref().unwrap().base_color.clone().unwrap();
    binding.role = TextureRole::LinearData;
    binding.uv_attribute = Some(Id(101));
    material.pbr.as_mut().unwrap().displacement = Some(displacement::Displacement {
        height: displacement::Height::Image {
            binding,
            scale_meters: 0.1,
            bias_meters: 0.,
        },
        subdivisions: 1,
        max_vertices: 128,
    });
    d.execute(
        &fixtures::principal(),
        &fixtures::request(
            &d,
            "named-uv:displace:01",
            vec![Command::PutMaterial {
                material: Box::new(material),
            }],
        )
        .unwrap(),
    )
    .unwrap();
    let before = canonical(&d).unwrap();
    let scene = Evaluator::default().evaluate(d.snapshot()).unwrap();
    assert_eq!(scene.instances.len(), 1);
    let g = &scene.instances[0].geometry;
    assert_eq!(
        g.uv_attributes,
        vec![
            Id(2),
            Id(101),
            Id(102),
            Id(103),
            Id(104),
            Id(105),
            Id(106),
            Id(107)
        ]
    );
    let slot = g
        .uv_attributes
        .iter()
        .position(|id| *id == Id(101))
        .unwrap();
    for tri in &g.triangles {
        for (p, uv) in tri.positions.iter().zip(tri.uv_sets[slot]) {
            assert!((f64::from(uv.x) - (p.y + 1.) / 2.).abs() < 1e-7);
            assert!((f64::from(uv.y) - (p.x + 1.) / 2.).abs() < 1e-7);
            let byte = match (uv.x < 0.5, uv.y < 0.5) {
                (true, true) => 255,
                (false, true) => 20,
                (true, false) => 30,
                (false, false) => 240,
            };
            assert!((p.z - f64::from(byte) / 255. * 0.1).abs() < 1e-8);
        }
    }
    assert!(
        scene
            .displacements
            .iter()
            .all(|r| r.approximation.starts_with("uniform-named-uv-v1"))
    );
    assert_eq!(canonical(&d).unwrap(), before);
    assert!(
        render(&scene, &settings(), || false)
            .unwrap()
            .linear
            .iter()
            .flatten()
            .all(|v| v.is_finite())
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn missing_uv_sets_bad_selectors_and_old_versions_fail_atomically() {
    for value in [json!(8), json!(-1), json!(null), json!("1")] {
        let mut v = source();
        v["materials"][0]["emissiveTexture"]["texCoord"] = value;
        assert!(imported(&v).is_err());
    }
    let mut v = source();
    v["meshes"][0]["primitives"][0]["attributes"]
        .as_object_mut()
        .unwrap()
        .remove("TEXCOORD_7");
    assert_eq!(imported(&v).err().unwrap().code, "reference");
    let mut d = document(&source());
    let before = canonical(&d).unwrap();
    let mut material = d
        .snapshot()
        .materials
        .values()
        .find(|m| m.pbr.as_ref().is_some_and(|p| p.emission.is_some()))
        .unwrap()
        .clone();
    material
        .pbr
        .as_mut()
        .unwrap()
        .emission
        .as_mut()
        .unwrap()
        .uv_attribute = Some(Id(999));
    let request = fixtures::request(
        &d,
        "named-uv:invalid:01",
        vec![Command::PutMaterial {
            material: Box::new(material),
        }],
    )
    .unwrap();
    assert_eq!(
        d.execute(&fixtures::principal(), &request)
            .unwrap_err()
            .code,
        "reference"
    );
    assert_eq!(canonical(&d).unwrap(), before);
    let mut old = d.snapshot().clone();
    old.version = 11;
    assert!(old.validate().is_err());
    assert_eq!(
        Evaluator::default()
            .evaluate_with_cancel(d.snapshot(), || true)
            .unwrap_err()
            .code,
        "cancelled"
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn named_uv_alpha_and_bake_follow_the_material_selector() {
    let d = document(&source());
    let scene = Evaluator::default().evaluate(d.snapshot()).unwrap();
    let request = products::Bake {
        name: "emission".into(),
        entity: scene.instances[0].id,
        width: 8,
        height: 8,
        pass: products::BakePass::Emission,
    };
    let baked = products::bake(&scene, &request, &scene.revision, || false).unwrap();
    for y in 0..8 {
        for x in 0..8 {
            let bytes = match (y < 4, x < 4) {
                (true, true) => [255, 30, 20],
                (false, true) => [20, 255, 30],
                (true, false) => [30, 20, 255],
                (false, false) => [240; 3],
            };
            let expected = Vec3::from_array(bytes.map(linear)) * 0.4;
            assert!(Vec3::from_array(baked.rgb[y * 8 + x]).distance(expected) < 1e-6);
        }
    }
    let mut v = source();
    let m = &mut v["materials"][0];
    m["alphaMode"] = json!("MASK");
    m["emissiveFactor"] = json!([0, 1, 0]);
    m["pbrMetallicRoughness"]["baseColorFactor"] = json!([0, 0, 0, 1]);
    m["pbrMetallicRoughness"]["baseColorTexture"]["texCoord"] = json!(1);
    m.as_object_mut().unwrap().remove("emissiveTexture");
    let d = document(&v);
    let scene = Evaluator::default().evaluate(d.snapshot()).unwrap();
    let mut s = settings();
    s.light.intensity = [0.; 3];
    s.environment = [0.; 3];
    s.samples = 1;
    s.max_depth = 1;
    let image = render(&scene, &s, || false).unwrap();
    for (i, p) in image.linear.iter().enumerate() {
        assert_eq!(*p, if i % 16 < 8 { [0.; 3] } else { [0., 1., 0.] });
        assert_eq!(image.objects[i].is_some(), i % 16 >= 8);
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn named_uv_primary_footprint_selects_its_own_mip() {
    let d = document(&source());
    let mut scene = Evaluator::default().evaluate(d.snapshot()).unwrap();
    let inst = &mut scene.instances[0];
    inst.material
        .pbr
        .as_mut()
        .unwrap()
        .emission
        .as_mut()
        .unwrap()
        .sampler
        .min = MinFilter::LinearMipLinear;
    let slot = inst
        .geometry
        .uv_attributes
        .iter()
        .position(|id| *id == Id(101))
        .unwrap();
    for tri in &mut std::sync::Arc::make_mut(&mut inst.geometry).triangles {
        for uv in &mut tri.uv_sets[slot] {
            *uv *= 16.;
        }
    }
    let s = settings();
    let ray = |x, y| s.camera.ray(x, y, s.width, s.height).unwrap();
    let hit = scene.intersect(ray(2.25, 3.25), 0.1, 5.).unwrap();
    let actual = shading(&scene, &hit, [ray(3.25, 3.25), ray(2.25, 4.25)]);
    let expected = (1. + linear(20) + linear(30) + linear(240)) / 4. * 0.4;
    assert!(
        actual.emission.distance(Vec3::splat(expected)) < 1e-6,
        "{:?} vs {expected}",
        actual.emission
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn geometry_export_reports_each_omitted_uv_set() {
    let d = document(&source());
    let mesh = d.snapshot().meshes.values().next().unwrap();
    let (_, obj) = interchange::export_obj(mesh).unwrap();
    let (_, _, gltf) = interchange::export_gltf(mesh).unwrap();
    for report in [obj, gltf] {
        let extra = report
            .losses
            .iter()
            .filter(|s| s.starts_with("additional UV attribute"))
            .collect::<Vec<_>>();
        assert_eq!(extra.len(), 7);
        for name in ["uv_1", "uv_2", "uv_3", "uv_4", "uv_5", "uv_6", "uv_7"] {
            assert!(extra.iter().any(|s| s.contains(name)));
        }
        assert!(!extra.iter().any(|s| s.contains("uv_0")));
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn source_indexed_sets_reject_gaps_missing_zero_and_noncanonical_suffixes() {
    for missing in ["TEXCOORD_0", "TEXCOORD_3", "TEXCOORD_6"] {
        let mut v = source();
        v["meshes"][0]["primitives"][0]["attributes"]
            .as_object_mut()
            .unwrap()
            .remove(missing);
        assert_eq!(imported(&v).err().unwrap().code, "gltf_scene");
    }
    for key in [
        "TEXCOORD_01",
        "TEXCOORD_+1",
        "TEXCOORD_999999999999999999999999",
        "JOINTS_1",
        "WEIGHTS_1",
        "COLOR_1",
    ] {
        let mut v = source();
        v["meshes"][0]["primitives"][0]["attributes"][key] = json!(0);
        assert_eq!(imported(&v).err().unwrap().code, "gltf_scene");
    }
    assert!(imported(&source()).is_ok());
}
