use render_core::{
    document::*,
    render::*,
    scattering::{Model, Opacity, Surface},
    *,
};
pub struct Case {
    pub name: &'static str,
    pub document: Document,
    pub settings: Settings,
    pub expected: [f32; 3],
}
pub fn cases() -> Vec<Case> {
    let mut cases = vec![];
    for (name, variant, ior) in [
        ("single-ior1", "single", 1.),
        ("single-ior1_5", "single", 1.5),
        ("single-ior3", "single", 3.),
        ("slab-ior1", "slab", 1.),
        ("slab-ior1_5", "slab", 1.5),
        ("slab-ior3", "slab", 3.),
        ("tilted-frame", "tilted", 1.5),
        ("mirror-x", "single", 1.5),
        ("mirror-z", "single", 1.5),
        ("snell", "single", 1.5),
        ("tir", "single", 1.5),
        ("grazing", "single", 1.5),
        ("near-clipped", "single", 1.5),
        ("far-clipped", "single", 1.5),
        ("mask-clear", "single", 1.5),
        ("blend-half", "single", 1.5),
    ] {
        let bytes: &[u8] = match variant {
            "slab" => include_bytes!("../../../../fixtures/gpu-dielectric/data/slab.glb"),
            "tilted" => include_bytes!("../../../../fixtures/gpu-dielectric/data/tilted.glb"),
            _ => include_bytes!("../../../../fixtures/gpu-dielectric/data/single.glb"),
        };
        let mut document = Document::new(Snapshot::empty(Id(10090))).unwrap();
        let imported = gltf_scene::import_pbr_glb(
            bytes,
            Id(10090),
            &gltf_scene::PbrPolicy {
                allow_approximations: true,
            },
        )
        .unwrap();
        document
            .execute(
                &fixtures::principal(),
                &fixtures::request(&document, "dielectric:analytic:import", imported.commands)
                    .unwrap(),
            )
            .unwrap();
        let mut settings = fixtures::settings();
        settings.width = 1;
        settings.height = 1;
        settings.samples = 16;
        settings.max_depth = if variant == "slab" { 3 } else { 2 };
        settings.environment = [0.; 3];
        settings.light.intensity = [0.; 3];
        settings.camera.position = [0., 0., 1.];
        settings.camera.target = [0.; 3];
        settings.camera.lens = Some(cameras::Lens::Orthographic {
            xmag: 1e-7,
            ymag: 1e-7,
            near: 0.01,
            far: 10.,
        });
        let mut commands = vec![];
        let glass_root = document
            .snapshot()
            .entities
            .iter()
            .find(|e| e.name == "glass-front")
            .unwrap()
            .id;
        let glass_id = document
            .snapshot()
            .entities
            .iter()
            .find(|e| e.parent == Some(glass_root) && e.mesh.is_some())
            .unwrap()
            .material
            .unwrap();
        for m in document
            .snapshot()
            .materials
            .values()
            .filter(|m| m.id == glass_id)
        {
            let mut m = m.clone();
            m.pbr.as_mut().unwrap().advanced = Some(Surface {
                model: Model::Dielectric { ior },
                opacity: match name {
                    "mask-clear" => Opacity::Mask {
                        factor: 0.,
                        cutoff: 0.5,
                    },
                    "blend-half" => Opacity::Blend { factor: 0.5 },
                    _ => Opacity::Opaque,
                },
            });
            commands.push(Command::PutMaterial {
                material: Box::new(m),
            });
        }
        let front = document
            .snapshot()
            .entities
            .iter()
            .find(|e| e.name == "glass-front")
            .unwrap();
        if name.starts_with("mirror") {
            let mut transform = front.transform.clone();
            let axis = if name == "mirror-x" { 0 } else { 2 };
            transform.columns[axis][axis] *= -1.;
            commands.push(Command::SetTransform {
                entity: front.id,
                transform,
            });
        }
        if name == "snell" || name == "tir" || name == "grazing" {
            let emitter = document
                .snapshot()
                .entities
                .iter()
                .find(|e| e.name == "emitter-target")
                .unwrap();
            let mut transform = emitter.transform.clone();
            transform.columns[0][0] = 0.05;
            if name == "snell" {
                settings.camera.position = [-0.5 / (0.75_f64).sqrt(), 0., 1.];
                // Snell sin(theta_t)=1/3, hence x at z=-1 is1/sqrt(8).
                transform.columns[3][0] = 1. / 8_f64.sqrt();
            } else if name == "grazing" {
                settings.camera.position = [-0.5 * 0.99_f64.sqrt() / 0.1, 0., 0.5];
                transform.columns[3][0] = (0.99_f64 / 2.25).sqrt() / (1. - 0.99_f64 / 2.25).sqrt();
            } else {
                settings.camera.position = [-0.5 * 0.9 / 0.19_f64.sqrt(), 0., -0.5];
                transform.columns[3][0] = 0.9 / 0.19_f64.sqrt();
            }
            commands.push(Command::SetTransform {
                entity: emitter.id,
                transform,
            });
        }
        if let Some(cameras::Lens::Orthographic { near, far, .. }) = settings.camera.lens.as_mut() {
            if name == "near-clipped" {
                *near = 1.1;
            }
            if name == "far-clipped" {
                *far = 0.9;
            }
        }
        commands.push(Command::SetRenderSettings {
            settings: settings.clone(),
        });
        document
            .execute(
                &fixtures::principal(),
                &fixtures::request(&document, "dielectric:analytic:recipe", commands).unwrap(),
            )
            .unwrap();
        let fresnel = if name == "grazing" {
            0.5715925203424491
        } else if name == "snell" {
            0.04152262597582152
        } else {
            ((ior - 1.) / (ior + 1.)).powi(2)
        };
        let count = (0..settings.samples)
            .filter(|&sample| {
                render_core::render::random(0, sample, 6, settings.seed) >= fresnel
                    && (variant != "slab"
                        || render_core::render::random(0, sample, 22, settings.seed) >= fresnel)
            })
            .count();
        let tint = [0.8_f32, 0.9, 1.];
        let emission = [0.75_f64, 0.5, 0.25];
        let expected = std::array::from_fn(|i| {
            if name == "far-clipped" {
                return 0.;
            }
            if ["tir", "near-clipped", "mask-clear"].contains(&name) {
                return emission[i] as f32;
            }
            if name == "blend-half" {
                let total = (0..settings.samples)
                    .map(|sample| {
                        if render_core::render::random(0, sample, 1024, settings.seed) >= 0.5 {
                            1.
                        } else if render_core::render::random(0, sample, 6, settings.seed)
                            >= fresnel
                        {
                            f64::from(tint[i]) / (ior * ior)
                        } else {
                            0.
                        }
                    })
                    .sum::<f64>();
                return (emission[i] * total / f64::from(settings.samples)) as f32;
            }
            let tint = f64::from(tint[i]);
            let weight = if variant == "slab" {
                tint * tint
            } else if name == "mirror-z" {
                tint * ior * ior
            } else {
                tint / (ior * ior)
            };
            (emission[i] * weight * count as f64 / f64::from(settings.samples)) as f32
        });
        cases.push(Case {
            name,
            document,
            settings,
            expected,
        });
    }
    cases
}
