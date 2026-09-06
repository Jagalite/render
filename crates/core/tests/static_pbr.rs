use glam::{DVec3, Vec2, Vec3};
use render_core::{
    agent::{self, Operation, Session},
    document::*,
    gltf_scene::*,
    pbr,
    render::*,
    textures::*,
    *,
};
use serde_json::{Value, json};
const GLB: &[u8] = include_bytes!("../../../fixtures/static-pbr/BoxTextured/BoxTextured.glb");
const GLTF: &[u8] = include_bytes!("../../../fixtures/static-pbr/BoxTextured/BoxTextured.gltf");
const BIN: &[u8] = include_bytes!("../../../fixtures/static-pbr/BoxTextured/BoxTextured0.bin");
const PNG: &[u8] = include_bytes!("../../../fixtures/static-pbr/BoxTextured/CesiumLogoFlat.png");
fn settings() -> Settings {
    let mut s = fixtures::settings();
    s.width = 16;
    s.height = 16;
    s.samples = 4;
    s.max_depth = 1;
    s.max_bytes = 128 * 1024 * 1024;
    s.camera = Camera {
        position: [2., 1.5, 3.],
        target: [0.; 3],
        up: [0., 1., 0.],
        vertical_fov_radians: 0.6,
        lens: None,
    };
    s
}
fn call(s: &mut Session, op: Operation) -> Result<Value> {
    s.dispatch(
        &fixtures::principal(),
        agent::Request {
            version: 0,
            operation: op,
        },
    )
}
fn imported() -> Session {
    let mut s = Session::new(Document::new(Snapshot::empty(Id(700))).unwrap());
    let base = s.document().snapshot().revision().unwrap();
    call(
        &mut s,
        Operation::ImportPbrGlb {
            bytes: GLB.to_vec(),
            policy: PbrPolicy {
                allow_approximations: true,
            },
            settings: settings(),
            base_revision: base,
            idempotency_key: "pbr:import:000001".into(),
        },
    )
    .unwrap();
    s
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn encoded_images_roles_wrap_filters_and_mips() {
    let asset = ImageAsset::from_encoded(Mime::Png, PNG.to_vec()).unwrap();
    assert_eq!(asset.decode().unwrap().dimensions(), (256, 256));
    assert!(ImageAsset::from_encoded(Mime::Jpeg, PNG.to_vec()).is_err());
    assert!(ImageAsset::from_encoded(Mime::Png, PNG[..32].to_vec()).is_err());
    let mut invalid = asset.clone();
    invalid.width = 1;
    assert!(invalid.decode().is_err());
    let mut jpeg = vec![];
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 100)
        .encode(
            &[128, 64, 32].repeat(16),
            4,
            4,
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
    let jpeg = ImageAsset::from_encoded(Mime::Jpeg, jpeg).unwrap();
    let pixel = jpeg.decode().unwrap().get_pixel(1, 1).0;
    for (a, b) in pixel[..3].iter().zip([128i16, 64, 32]) {
        assert!((i16::from(*a) - b).abs() <= 3);
    }
    assert!(ImageAsset::from_encoded(Mime::Jpeg, jpeg.encoded[..20].to_vec()).is_err());
    let mut oversized = vec![];
    image::codecs::png::PngEncoder::new(&mut oversized)
        .write_image(&vec![0; 2049 * 4], 2049, 1, image::ExtendedColorType::Rgba8)
        .unwrap();
    assert!(ImageAsset::from_encoded(Mime::Png, oversized).is_err());
    let img = image::RgbaImage::from_raw(2, 1, vec![0, 0, 0, 255, 128, 128, 128, 255]).unwrap();
    let linear = Pyramid::new(&img, TextureRole::LinearData);
    let color = Pyramid::new(&img, TextureRole::SrgbColor);
    let mut sampler = Sampler {
        mag: Filter::Nearest,
        min: MinFilter::Nearest,
        ..Sampler::default()
    };
    let sample =
        |p: &Pyramid, s: &Sampler, x| p.sample(s, Vec2::new(x, 0.5), Vec2::ZERO, Vec2::ZERO).x;
    assert!((sample(&linear, &sampler, 0.75) - 128. / 255.).abs() < 1e-6);
    assert!((sample(&color, &sampler, 0.75) - 0.2158605).abs() < 1e-6);
    assert_eq!(
        sample(&linear, &sampler, -0.25),
        sample(&linear, &sampler, 0.75)
    );
    sampler.wrap_s = Wrap::Clamp;
    assert_eq!(sample(&linear, &sampler, -0.25), 0.);
    sampler.wrap_s = Wrap::Mirror;
    assert_eq!(
        sample(&linear, &sampler, 1.25),
        sample(&linear, &sampler, 0.75)
    );
    sampler.mag = Filter::Linear;
    assert!((sample(&color, &sampler, 0.5) - 0.10793025).abs() < 1e-6);
    for min in [
        MinFilter::Nearest,
        MinFilter::Linear,
        MinFilter::NearestMipNearest,
        MinFilter::LinearMipNearest,
        MinFilter::NearestMipLinear,
        MinFilter::LinearMipLinear,
    ] {
        sampler.min = min;
        let p = linear.sample(
            &sampler,
            Vec2::new(0.25, 0.5),
            Vec2::new(1., 0.),
            Vec2::ZERO,
        );
        let expected = if matches!(min, MinFilter::Nearest | MinFilter::Linear) {
            0.
        } else {
            64. / 255.
        };
        assert!((p.x - expected).abs() < 1e-6);
    }
    let npot =
        image::RgbaImage::from_raw(3, 1, vec![0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255, 255])
            .unwrap();
    assert!(
        (Pyramid::new(&npot, TextureRole::LinearData).levels[1].rgba[0][0] - 1. / 3.).abs() < 1e-6
    );
    let surface = pbr::Surface {
        normal: Some(Binding {
            image: asset.content_id().unwrap(),
            role: TextureRole::SrgbColor,
            sampler,
        }),
        ..Default::default()
    };
    assert_eq!(surface.validate().unwrap_err().code, "texture_role");
}
use image::ImageEncoder;
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn ggx_analytic_reciprocity_energy_and_sampling() {
    let n = DVec3::Z;
    let color = Vec3::splat(0.5);
    let pi = std::f64::consts::PI;
    // Independent normal-incidence equation at roughness 1: D=1/pi,V=1/4,F=.04.
    let expected = 0.96 * 0.5 / pi + 0.04 / (4. * pi);
    assert!((pbr::brdf(color, 0., 1., n, n, n).x - expected).abs() < 1e-12);
    assert!((pbr::brdf(color, 1., 1., n, n, n).x - 0.5 / (4. * pi)).abs() < 1e-12);
    assert_eq!(pbr::brdf(color, 0., 1., n, n, -n), DVec3::ZERO);
    assert_eq!(pbr::distribution(1., 0.), pbr::distribution(1., 0.05));
    for color in [Vec3::splat(0.5), Vec3::ONE] {
        for rough in [0.25, 0.5, 1.] {
            for metal in [0., 1.] {
                let view = DVec3::new(0.6, 0., 0.8);
                let mut integral = DVec3::ZERO;
                let mut pdf_integral = 0.;
                let mut ndf = 0.;
                let steps = 160;
                for z in 0..steps {
                    let cos = (f64::from(z) + 0.5) / f64::from(steps);
                    ndf +=
                        pbr::distribution(cos, f64::from(rough)) * cos * 2. * pi / f64::from(steps);
                    for a in 0..steps {
                        let phi = (f64::from(a) + 0.5) * 2. * pi / f64::from(steps);
                        let sin = (1. - cos * cos).sqrt();
                        let l = DVec3::new(sin * phi.cos(), sin * phi.sin(), cos);
                        let f = pbr::brdf(color, metal, rough, n, view, l);
                        assert!((f - pbr::brdf(color, metal, rough, n, l, view)).length() < 1e-10);
                        integral += f * cos * (2. * pi / f64::from(steps * steps));
                        pdf_integral +=
                            pbr::pdf(n, view, l, rough) * 2. * pi / f64::from(steps * steps);
                    }
                }
                // Sharp NDF requires finer integration; use exact CDF integral for sharp lobe separately below.
                if rough >= 0.5 {
                    assert!((ndf - 1.).abs() < 0.004);
                }
                assert!(integral.min_element() >= 0. && integral.max_element() <= 1.001);
                let mut mean = DVec3::ZERO;
                let mut accepted = 0.;
                let count = 20000;
                for i in 0..count {
                    let l = pbr::sample(
                        n,
                        view,
                        rough,
                        random(i, 0, 4, 42),
                        random(i, 0, 2, 42),
                        random(i, 0, 3, 42),
                    );
                    let pdf = pbr::pdf(n, view, l, rough);
                    if pdf > 0. {
                        accepted += 1.;
                        mean += pbr::brdf(color, metal, rough, n, view, l) * n.dot(l) / pdf;
                    }
                }
                assert!((accepted / f64::from(count) - pdf_integral).abs() < 0.015);
                assert!((mean / f64::from(count) - integral).abs().max_element() < 0.015);
            }
        }
    }
    // Closed form projected GGX CDF integrates to one for every alpha, including the floor.
    for rough in [0.0_f64, 0.05, 0.25, 1.] {
        let a2 = rough.max(0.05_f64).powi(4);
        let cdf = |x: f64| a2 * x * x / (1. + (a2 - 1.) * x * x);
        assert!((cdf(1.) - cdf(0.) - 1.).abs() < 1e-10);
    }
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn static_pbr_transaction_roundtrip_render_and_failures() {
    let mut session = imported();
    let snapshot = session.document().snapshot();
    assert_eq!(snapshot.version, 2);
    assert_eq!(snapshot.images.len(), 1);
    let encoded = canonical(session.document()).unwrap();
    let reopened: Document = serde_json::from_slice(&encoded).unwrap();
    reopened.snapshot().validate().unwrap();
    assert_eq!(
        snapshot.revision().unwrap(),
        reopened.snapshot().revision().unwrap()
    );
    let scene = Evaluator::default().evaluate(reopened.snapshot()).unwrap();
    let image = render(&scene, &settings(), || false).unwrap();
    assert!(image.objects.iter().filter(|v| v.is_some()).count() > 32);
    assert!(
        image
            .linear
            .iter()
            .flatten()
            .all(|v| v.is_finite() && *v >= 0.)
    );
    assert_eq!(
        render(&scene, &settings(), || true).unwrap_err().code,
        "cancelled"
    );
    let mut s = settings();
    s.max_depth = 17;
    assert_eq!(
        render(&scene, &s, || false).unwrap_err().code,
        "render_settings"
    );
    let mut overflow_scene = scene.clone();
    for instance in &mut overflow_scene.instances {
        instance.material.emission = [f32::MAX; 3];
    }
    assert_eq!(
        render(&overflow_scene, &settings(), || false)
            .unwrap_err()
            .code,
        "numerics"
    );
    let base = snapshot.revision().unwrap();
    let before = canonical(session.document()).unwrap();
    let bad = Operation::ImportPbrGlb {
        bytes: GLB.to_vec(),
        policy: PbrPolicy {
            allow_approximations: true,
        },
        settings: settings(),
        base_revision: "stale".into(),
        idempotency_key: "pbr:import:stale:01".into(),
    };
    assert!(call(&mut session, bad).is_err());
    assert_eq!(canonical(session.document()).unwrap(), before);
    call(
        &mut session,
        Operation::Branch {
            branch: "pbr".into(),
            base_revision: base.clone(),
        },
    )
    .unwrap();
    assert!(
        session
            .render_input(&fixtures::principal(), "pbr", "stale")
            .is_err()
    );
    assert_eq!(
        session
            .preview(&fixtures::principal(), "pbr", &base, || true)
            .unwrap_err()
            .code,
        "cancelled"
    );
    assert!(
        import_pbr_glb(
            &GLB[..64],
            Id(1),
            &PbrPolicy {
                allow_approximations: true
            }
        )
        .is_err()
    );
    assert!(
        import_pbr_glb(
            GLB,
            Id(1),
            &PbrPolicy {
                allow_approximations: false
            }
        )
        .is_err()
    );
    let json: Value = serde_json::from_slice(GLTF).unwrap();
    let imported = import_pbr_scene(
        GLTF,
        &[BIN.to_vec()],
        &[PNG.to_vec()],
        Id(700),
        &PbrPolicy {
            allow_approximations: true,
        },
    )
    .unwrap();
    assert!(
        imported
            .commands
            .iter()
            .any(|c| matches!(c, Command::PutImage { .. }))
    );
    for (pointer, value) in [
        ("/materials/0/alphaMode", json!("UNKNOWN")),
        (
            "/materials/0/pbrMetallicRoughness/baseColorTexture/texCoord",
            json!(1),
        ),
        ("/samplers/0/wrapS", json!(0)),
        ("/nodes/0/skin", json!(0)),
        ("/accessors/0/count", json!(999999)),
    ] {
        let mut invalid = json.clone();
        let target = pointer.rsplit_once('/').unwrap();
        invalid.pointer_mut(target.0).unwrap()[target.1] = value;
        assert!(
            import_pbr_scene(
                &serde_json::to_vec(&invalid).unwrap(),
                &[BIN.to_vec()],
                &[PNG.to_vec()],
                Id(1),
                &PbrPolicy {
                    allow_approximations: true
                }
            )
            .is_err(),
            "{pointer}"
        );
    }
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn authored_camera_selection_clipping_and_protection() {
    let mut json: Value = serde_json::from_slice(GLTF).unwrap();
    json["cameras"] =
        json!([{"type":"perspective","perspective":{"yfov":0.7,"znear":0.1,"zfar":10.}}]);
    let nodes = json["nodes"].as_array_mut().unwrap();
    let index = nodes.len();
    nodes.push(json!({"camera":0,"translation":[0.,0.,3.]}));
    json["scenes"][0]["nodes"]
        .as_array_mut()
        .unwrap()
        .push(json!(index));
    let mut session = Session::new(Document::new(Snapshot::empty(Id(701))).unwrap());
    let base = session.document().snapshot().revision().unwrap();
    call(
        &mut session,
        Operation::ImportPbrScene {
            json: json.clone(),
            buffers: vec![BIN.to_vec()],
            images: vec![PNG.to_vec()],
            policy: PbrPolicy {
                allow_approximations: true,
            },
            settings: settings(),
            base_revision: base,
            idempotency_key: "camera:import:0001".into(),
        },
    )
    .unwrap();
    let snapshot = session.document().snapshot();
    assert!(
        snapshot
            .render_settings
            .as_ref()
            .unwrap()
            .camera
            .lens
            .is_none()
    );
    let camera = *snapshot.cameras.keys().next().unwrap();
    let protected = agent::protected_digest(snapshot).unwrap();
    let base = snapshot.revision().unwrap();
    let op = Operation::SelectCamera {
        entity: camera,
        base_revision: base,
        idempotency_key: "camera:select:0001".into(),
    };
    let receipt = call(&mut session, op.clone()).unwrap();
    assert_eq!(receipt, call(&mut session, op.clone()).unwrap());
    let mut changed = session
        .document()
        .snapshot()
        .render_settings
        .clone()
        .unwrap();
    changed.environment = [0.8; 3];
    let request = fixtures::request(
        session.document(),
        "camera:later:00001",
        vec![Command::SetRenderSettings { settings: changed }],
    )
    .unwrap();
    session
        .execute_root(&fixtures::principal(), &request)
        .unwrap();
    assert_eq!(receipt, call(&mut session, op).unwrap());
    assert_ne!(
        protected,
        agent::protected_digest(session.document().snapshot()).unwrap()
    );
    let cam = session.document().snapshot().camera(camera).unwrap();
    assert_eq!(cam.position, [0., 0., 3.]);
    let ray = cam.ray(8., 8., 16, 16).unwrap();
    assert_eq!(ray.direction, DVec3::NEG_Z);
    assert_eq!(cam.clip(ray), (0.1, 10.));
    let mut ortho = cam.clone();
    ortho.lens = Some(cameras::Lens::Orthographic {
        xmag: 2.,
        ymag: 1.,
        near: 0.,
        far: 5.,
    });
    let a = ortho.ray(0., 0., 16, 16).unwrap();
    let b = ortho.ray(16., 16., 16, 16).unwrap();
    assert_eq!(a.direction, b.direction);
    assert_eq!((a.origin - b.origin).to_array(), [-4., 2., 0.]);
    let mut doc = session.document().clone();
    let mut settings = doc.snapshot().render_settings.clone().unwrap();
    settings.camera.lens = Some(cameras::Lens::Perspective {
        vertical_fov_radians: 0.7,
        aspect_ratio: None,
        near: 0.1,
        far: Some(1.),
    });
    let request = fixtures::request(
        &doc,
        "camera:clip:000001",
        vec![Command::SetRenderSettings {
            settings: settings.clone(),
        }],
    )
    .unwrap();
    doc.execute(&fixtures::principal(), &request).unwrap();
    let image = render(
        &Evaluator::default().evaluate(doc.snapshot()).unwrap(),
        &settings,
        || false,
    )
    .unwrap();
    assert!(image.objects.iter().all(Option::is_none));
    json["nodes"][index]["scale"] = json!([2., 1., 1.]);
    let imported = import_pbr_scene(
        &serde_json::to_vec(&json).unwrap(),
        &[BIN.to_vec()],
        &[PNG.to_vec()],
        Id(701),
        &PbrPolicy {
            allow_approximations: true,
        },
    )
    .unwrap();
    let mut doc = Document::new(Snapshot::empty(Id(701))).unwrap();
    let request = fixtures::request(&doc, "camera:invalid:01", imported.commands).unwrap();
    assert_eq!(
        doc.execute(&fixtures::principal(), &request)
            .unwrap_err()
            .code,
        "unsupported_camera"
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn tangent_handedness_inverse_transpose_culling_and_jpeg_workflow() {
    use geometry::*;
    let mut mesh = Mesh::from_polygons(
        Positions::F32(vec![[-2., -2., 0.], [2., -2., 0.], [0., 2., 0.]]),
        &[vec![0, 1, 2]],
        &[],
    )
    .unwrap();
    for (name, id, values) in [
        ("normal", 1, AttributeValues::Vec3(vec![[0.6, 0., 0.8]; 3])),
        ("tangent", 2, AttributeValues::Vec3(vec![[1., 0., 0.]; 3])),
        ("tangent_sign", 3, AttributeValues::Scalar(vec![1.; 3])),
    ] {
        mesh.attributes.insert(
            name.into(),
            Attribute {
                id: Id(id),
                domain: Domain::Point,
                semantic: name.into(),
                transfer: Transfer::Nearest,
                values,
            },
        );
    }
    let mut bytes = vec![];
    image::codecs::png::PngEncoder::new(&mut bytes)
        .write_image(&[128, 255, 255, 255], 1, 1, image::ExtendedColorType::Rgba8)
        .unwrap();
    let image = ImageAsset::from_encoded(Mime::Png, bytes).unwrap();
    let mut material = Material::diffuse(Id(20), [0.5; 3]);
    material.pbr = Some(pbr::Surface {
        normal: Some(Binding {
            image: image.content_id().unwrap(),
            role: TextureRole::LinearData,
            sampler: Sampler::default(),
        }),
        ..Default::default()
    });
    let transform = Transform {
        operations: vec![TransformOp::Scale([-2., 3., 1.])],
        ..Default::default()
    };
    let entity = Entity {
        id: Id(21),
        name: "mirrored analytic normal".into(),
        parent: None,
        mesh: Some(mesh.content_id().unwrap()),
        material: Some(material.id),
        transform,
    };
    let mut doc = Document::new(Snapshot::empty(Id(99))).unwrap();
    let request = fixtures::request(
        &doc,
        "normal:analytic:01",
        vec![
            Command::PutImage { image },
            Command::PutMesh { mesh },
            Command::PutMaterial { material },
            Command::CreateEntity { entity },
        ],
    )
    .unwrap();
    doc.execute(&fixtures::principal(), &request).unwrap();
    let scene = Evaluator::default().evaluate(doc.snapshot()).unwrap();
    let ray = Ray {
        origin: DVec3::new(0., 0., 2.),
        direction: DVec3::NEG_Z,
    };
    let hit = scene.intersect(ray, 0.1, 10.).unwrap();
    let expected = DVec3::new(-0.3, 0., 0.8).normalize();
    assert!((hit.normal - expected).length() < 1e-7);
    assert_eq!(hit.tangent_sign, -1.);
    assert!(hit.tangent.dot(hit.normal).abs() < 1e-12);
    let surface = shading(&scene, &hit, [ray; 2]);
    let expected = (hit.tangent * (1. / 255.) + DVec3::Y + expected).normalize();
    assert!((surface.normal - expected).length() < 1e-7);
    assert!(
        scene
            .intersect(
                Ray {
                    origin: -ray.origin,
                    direction: -ray.direction
                },
                0.1,
                10.
            )
            .is_none()
    );
    let mut two_sided = scene.clone();
    two_sided.instances[0]
        .material
        .pbr
        .as_mut()
        .unwrap()
        .double_sided = true;
    let back_ray = Ray {
        origin: -ray.origin,
        direction: -ray.direction,
    };
    let back = two_sided.intersect(back_ray, 0.1, 10.).unwrap();
    let back_surface = shading(&two_sided, &back, [back_ray; 2]);
    assert!((back_surface.normal + surface.normal).length() < 1e-7);
    let mut called = 0;
    assert_eq!(
        render(&scene, &settings(), || {
            called += 1;
            called == 3
        })
        .unwrap_err()
        .code,
        "cancelled"
    );
    assert_eq!(called, 3);
    // JPEG is imported, authored, persisted and rendered, not only decoded in isolation.
    let mut jpeg = vec![];
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 100)
        .encode(
            &[128, 64, 32].repeat(16),
            4,
            4,
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
    let mut json: Value = serde_json::from_slice(GLTF).unwrap();
    json["images"][0]["uri"] = json!("caller-supplied.jpg");
    json["images"][0]["mimeType"] = json!("image/jpeg");
    let imported = import_pbr_scene(
        &serde_json::to_vec(&json).unwrap(),
        &[BIN.to_vec()],
        &[jpeg],
        Id(700),
        &PbrPolicy {
            allow_approximations: true,
        },
    )
    .unwrap();
    let mut doc = Document::new(Snapshot::empty(Id(700))).unwrap();
    let request = fixtures::request(&doc, "jpeg:workflow:001", imported.commands).unwrap();
    doc.execute(&fixtures::principal(), &request).unwrap();
    let restored: Document = serde_json::from_slice(&canonical(&doc).unwrap()).unwrap();
    let rendered = render(
        &Evaluator::default().evaluate(restored.snapshot()).unwrap(),
        &settings(),
        || false,
    )
    .unwrap();
    assert!(rendered.objects.iter().any(Option::is_some));
    assert!(
        rendered
            .linear
            .iter()
            .flatten()
            .all(|v| v.is_finite() && *v >= 0.)
    );
}
