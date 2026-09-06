//! Independently authored, deterministic fixtures for M07/M08 integration gates.
use crate::{curves::*, document::*, render::*, *};
pub fn geometry_document() -> Result<Document> {
    let mut document = Document::new(Snapshot::empty(Id(7000)))?;
    let control = |id, position| Control {
        id: Id(id),
        position,
        radius: 0.045,
        tilt: 0.,
    };
    let shapes = vec![
        Shape::Curves {
            curves: vec![Curve {
                id: Id(10),
                basis: Basis::CubicBezier,
                closed: false,
                controls: vec![
                    control(1, [-1.2, -0.5, 0.]),
                    control(2, [-1.8, 1.3, 0.]),
                    control(3, [-0.1, 1.3, 0.]),
                    control(4, [-0.6, -0.5, 0.]),
                ],
            }],
        },
        Shape::Points {
            points: vec![
                Point {
                    id: Id(11),
                    position: [0., 0.6, 0.],
                    radius: 0.17,
                },
                Point {
                    id: Id(12),
                    position: [0.15, 0., 0.],
                    radius: 0.23,
                },
                Point {
                    id: Id(13),
                    position: [0., -0.55, 0.],
                    radius: 0.15,
                },
            ],
        },
        Shape::Curves {
            curves: vec![Curve {
                id: Id(20),
                basis: Basis::RationalBSpline {
                    degree: 2,
                    knots: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
                    weights: vec![
                        1.,
                        std::f64::consts::FRAC_1_SQRT_2,
                        1.,
                        std::f64::consts::FRAC_1_SQRT_2,
                        1.,
                    ],
                },
                closed: false,
                controls: vec![
                    control(1, [1.55, 0., 0.]),
                    control(2, [1.55, 0.5, 0.]),
                    control(3, [1.05, 0.5, 0.]),
                    control(4, [0.55, 0.5, 0.]),
                    control(5, [0.55, 0., 0.]),
                ],
            }],
        },
    ];
    let mut commands = vec![];
    for (i, (shape, color)) in shapes
        .into_iter()
        .zip([[0.1, 0.7, 0.3], [0.8, 0.25, 0.05], [0.25, 0.25, 0.8]])
        .enumerate()
    {
        let asset = Asset {
            shape,
            tessellation: Tessellation {
                chord_error: 0.002,
                radial_error: 0.002,
                ..Default::default()
            },
        };
        let key = asset.content_id()?;
        let id = Id(7100 + i as u128);
        let material_id = Id(7200 + i as u128);
        let mut material = Material::diffuse(material_id, color);
        material.pbr = Some(pbr::Surface::default());
        material.roughness = 0.7;
        commands.extend([
            Command::PutGeometry { asset },
            Command::PutMaterial { material },
            Command::CreateEntity {
                entity: Entity {
                    id,
                    name: format!("M07 typed geometry {i}"),
                    parent: None,
                    mesh: None,
                    material: Some(material_id),
                    transform: Transform::default(),
                },
            },
            Command::SetGeometry {
                entity: id,
                asset: Some(key),
            },
        ]);
    }
    let mut settings = fixtures::settings();
    settings.width = 96;
    settings.height = 64;
    settings.samples = 8;
    settings.max_depth = 1;
    settings.max_bytes = 128 * 1024 * 1024;
    settings.environment = [0.3; 3];
    settings.light = PointLight {
        position: [1., 3., 3.],
        intensity: [15.; 3],
    };
    settings.camera = Camera {
        position: [0., 0.2, 4.],
        target: [0., 0.2, 0.],
        up: [0., 1., 0.],
        vertical_fov_radians: 0.55,
        lens: None,
    };
    commands.push(Command::SetRenderSettings { settings });
    let request = fixtures::request(&document, "m07:geometry:author:01", commands)?;
    document.execute(&fixtures::principal(), &request)?;
    Ok(document)
}

pub fn volume_document() -> Result<Document> {
    let mut document = geometry_document()?;
    let mut cells = vec![];
    for x in 0..6 {
        for y in 0..6 {
            for z in 0..4 {
                let p =
                    glam::DVec3::new(f64::from(x) - 2.5, f64::from(y) - 2.5, f64::from(z) - 1.5);
                if p.length() < 3.2 {
                    cells.push(volumes::Cell {
                        coordinate: [x, y, z],
                        density: 0.8 + (3.2 - p.length()) * 0.6,
                        emission: [0.06, 0.01, 0.002],
                    });
                }
            }
        }
    }
    let asset = volumes::Asset {
        origin: [-0.9, -0.7, 0.2],
        voxel_size: [0.3; 3],
        cells,
        absorption: [0.15, 0.7, 1.1],
        scattering: [0.4; 3],
        anisotropy: 0.25,
        max_step_meters: 0.1,
    };
    let key = asset.content_id()?;
    let mut settings = document
        .snapshot()
        .render_settings
        .clone()
        .expect("geometry settings");
    settings.width = 64;
    settings.height = 48;
    settings.samples = 4;
    let req = fixtures::request(
        &document,
        "m07:volume:author:01",
        vec![
            Command::PutVolume { asset },
            Command::CreateEntity {
                entity: Entity {
                    id: Id(7400),
                    name: "sparse amber smoke".into(),
                    parent: None,
                    mesh: None,
                    material: None,
                    transform: Transform::default(),
                },
            },
            Command::SetVolume {
                entity: Id(7400),
                asset: Some(key),
            },
            Command::SetRenderSettings { settings },
        ],
    )?;
    document.execute(&fixtures::principal(), &req)?;
    Ok(document)
}

pub fn groom_document() -> Result<Document> {
    use crate::{geometry::*, groom::*};
    let mut document = Document::new(Snapshot::empty(Id(7500)))?;
    let mesh = Mesh::from_polygons(
        Positions::F64(vec![
            [-0.8, 0., -0.7],
            [-0.8, 0., 0.7],
            [0.8, 0., 0.7],
            [0.8, 0., -0.7],
        ]),
        &[vec![0, 1, 2, 3]],
        &[],
    )?;
    let topology = topology(&mesh)?;
    let corners = mesh.triangles()?[0].map(|i| mesh.corner_ids[i as usize]);
    let root = |barycentric| Root {
        entity: Id(7600),
        topology: topology.clone(),
        corners,
        barycentric,
    };
    let curve = Curve {
        id: Id(1),
        basis: Basis::CubicBezier,
        closed: false,
        controls: vec![
            Control {
                id: Id(1),
                position: [0.; 3],
                radius: 0.016,
                tilt: 0.,
            },
            Control {
                id: Id(2),
                position: [0.05, 0., 0.25],
                radius: 0.014,
                tilt: 0.,
            },
            Control {
                id: Id(3),
                position: [0.12, 0., 0.5],
                radius: 0.01,
                tilt: 0.,
            },
            Control {
                id: Id(4),
                position: [0.2, 0., 0.65],
                radius: 0.006,
                tilt: 0.,
            },
        ],
    };
    let mut children = vec![];
    for i in 1..8 {
        for j in 1..(9 - i) {
            let a = f64::from(i) / 10.;
            let b = f64::from(j) / 10.;
            children.push(Child {
                id: Id(10 + children.len() as u128),
                guide: Id(1),
                root: root([a, b, 1. - a - b]),
                length_scale: 0.8 + f64::from((i + j) % 3) * 0.1,
                radius_scale: 0.8,
                twist: f64::from(i - j) * 0.2,
            });
        }
    }
    let groom = Groom {
        guides: vec![Guide {
            curve,
            root: root([0.8, 0.1, 0.1]),
        }],
        children,
        tessellation: Tessellation {
            chord_error: 0.004,
            radial_error: 0.004,
            ..Default::default()
        },
    };
    let mut settings = fixtures::settings();
    settings.width = 64;
    settings.height = 64;
    settings.samples = 8;
    settings.max_depth = 1;
    settings.environment = [0.3; 3];
    settings.camera = Camera {
        position: [1.5, 1.3, 2.5],
        target: [0., 0.25, 0.],
        up: [0., 1., 0.],
        vertical_fov_radians: 0.6,
        lens: None,
    };
    let req = fixtures::request(
        &document,
        "m07:groom:author:01",
        vec![
            Command::PutMesh { mesh: mesh.clone() },
            Command::PutMaterial {
                material: Material::diffuse(Id(7700), [0.2, 0.3, 0.25]),
            },
            Command::PutMaterial {
                material: Material::diffuse(Id(7701), [0.55, 0.15, 0.04]),
            },
            Command::CreateEntity {
                entity: Entity {
                    id: Id(7600),
                    name: "groom anchor".into(),
                    parent: None,
                    mesh: Some(mesh.content_id()?),
                    material: Some(Id(7700)),
                    transform: Transform::default(),
                },
            },
            Command::CreateEntity {
                entity: Entity {
                    id: Id(7601),
                    name: "guided strands".into(),
                    parent: None,
                    mesh: None,
                    material: Some(Id(7701)),
                    transform: Transform::default(),
                },
            },
            Command::SetGroom {
                entity: Id(7601),
                groom: Some(groom),
            },
            Command::SetRenderSettings { settings },
        ],
    )?;
    document.execute(&fixtures::principal(), &req)?;
    Ok(document)
}

pub fn character_document() -> Result<Document> {
    use crate::{animation::*, geometry::*, rigging::*};
    use std::collections::BTreeMap;
    let mut document = Document::new(Snapshot::empty(Id(8000)))?;
    let mut points = vec![];
    let mut faces = vec![];
    let mut weights = BTreeMap::new();
    // Original block-character fixture: torso, head, legs, static arm, bending arm.
    let boxes = [
        ([-0.24, -0.3, -0.15], [0.24, 0.7, 0.15], 0),
        ([-0.22, 0.75, -0.2], [0.22, 1.18, 0.2], 0),
        ([-0.23, -0.95, -0.12], [-0.04, -0.3, 0.12], 0),
        ([0.04, -0.95, -0.12], [0.23, -0.3, 0.12], 0),
        ([-0.85, 0.5, -0.1], [-0.24, 0.7, 0.1], 0),
        ([0.35, 0.55, -0.1], [0.85, 0.75, 0.1], 1),
        ([0.85, 0.55, -0.1], [1.35, 0.75, 0.1], 2),
    ];
    for (min, max, joint) in boxes {
        let start = points.len() as u32;
        for i in 0..8 {
            points.push(std::array::from_fn(|a| {
                if i & (1 << a) == 0 { min[a] } else { max[a] }
            }));
        }
        for f in [
            [0, 2, 3, 1],
            [4, 5, 7, 6],
            [0, 1, 5, 4],
            [2, 6, 7, 3],
            [0, 4, 6, 2],
            [1, 3, 7, 5],
        ] {
            faces.push(f.map(|i| start + i).to_vec());
        }
        // Mesh factory local IDs are assigned after creation below.
        for i in start..start + 8 {
            weights.insert(
                i,
                vec![Influence {
                    joint: Id(8100 + joint),
                    weight: 1.,
                }],
            );
        }
    }
    let mesh = Mesh::from_polygons(Positions::F64(points), &faces, &[])?;
    let authored_mesh = mesh;
    let (mesh, _) = modeling::apply(
        &authored_mesh,
        &modeling::Operation::Triangulate,
        &modeling::Budget::default(),
        || false,
    )?;
    let topology = crate::groom::topology(&mesh)?;
    let mut rig = Rig {
        joints: vec![
            Joint {
                id: Id(8100),
                parent: None,
                rest: Transform::default(),
                inverse_bind: Transform::default(),
            },
            Joint {
                id: Id(8101),
                parent: Some(Id(8100)),
                rest: Transform::translation(0.35, 0.65, 0.),
                inverse_bind: Transform::translation(-0.35, -0.65, 0.),
            },
            Joint {
                id: Id(8102),
                parent: Some(Id(8101)),
                rest: Transform::translation(0.5, 0., 0.),
                inverse_bind: Transform::translation(-0.85, -0.65, 0.),
            },
            Joint {
                id: Id(8103),
                parent: Some(Id(8102)),
                rest: Transform::translation(0.5, 0., 0.),
                inverse_bind: Transform::translation(-1.35, -0.65, 0.),
            },
        ],
        constraints: vec![],
    };
    rig.constraints.push(Constraint::TwoBoneIk {
        id: Id(8150),
        root: Id(8101),
        mid: Id(8102),
        tip: Id(8103),
        target: Goal::Entity { entity: Id(8500) },
        pole: [0.4, 2., 0.],
        tolerance: 1e-8,
    });
    let skin = Skin {
        inverse_binds: BTreeMap::new(),
        rig: Id(8200),
        topology: topology.clone(),
        mesh_bind: Transform::default(),
        weights: weights
            .into_iter()
            .map(|(i, w)| (mesh.point_ids[i as usize], w))
            .collect(),
    };
    let morph = Morph {
        default_weight: 0.,
        id: Id(8300),
        topology,
        offsets: (8..16)
            .map(|i| {
                let p = mesh.positions.get(i);
                (mesh.point_ids[i], [p.x * 0.15, -(p.y - 0.75) * 0.12, 0.])
            })
            .collect(),
    };
    let key = |n, d, value| Key {
        time: Time::new(n, d).expect("fixture time"),
        value,
        incoming: None,
        outgoing: None,
    };
    let clip = Clip {
        id: Id(8400),
        start: Time::new(0, 1)?,
        end: Time::new(1, 1)?,
        extrapolation: Extrapolation::Clamp,
        remap: TimeMap {
            rate: Time::new(1, 1)?,
            offset: Time::new(0, 1)?,
        },
        tracks: vec![
            Track {
                id: Id(8401),
                target: Target::Entity { entity: Id(8500) },
                property: Property::Translation,
                interpolation: Interpolation::Linear,
                keys: vec![
                    key(0, 1, Value::Vector([0.; 3])),
                    key(1, 2, Value::Vector([-0.2, 0.35, 0.])),
                    key(1, 1, Value::Vector([0.; 3])),
                ],
            },
            Track {
                id: Id(8402),
                target: Target::Morph {
                    entity: Id(8201),
                    target: Id(8300),
                },
                property: Property::MorphWeight,
                interpolation: Interpolation::Linear,
                keys: vec![
                    key(0, 1, Value::Scalar(0.)),
                    key(1, 2, Value::Scalar(1.)),
                    key(1, 1, Value::Scalar(0.)),
                ],
            },
            Track {
                id: Id(8403),
                target: Target::Entity { entity: Id(8200) },
                property: Property::Translation,
                interpolation: Interpolation::Linear,
                keys: vec![
                    key(0, 1, Value::Vector([-0.15, 0., 0.])),
                    key(1, 1, Value::Vector([0.15, 0., 0.])),
                ],
            },
        ],
    };
    let animation = State {
        clips: BTreeMap::from([(clip.id, clip)]),
        rigs: BTreeMap::from([(Id(8200), rig)]),
        skins: BTreeMap::from([(Id(8201), skin)]),
        morphs: BTreeMap::from([(Id(8201), vec![morph])]),
    };
    let mut settings = fixtures::settings();
    settings.width = 64;
    settings.height = 64;
    settings.samples = 4;
    settings.max_depth = 1;
    settings.environment = [0.35; 3];
    settings.camera = Camera {
        position: [1.7, 1.1, 4.5],
        target: [0.1, 0.1, 0.],
        up: [0., 1., 0.],
        vertical_fov_radians: 0.6,
        lens: None,
    };
    let req = fixtures::request(
        &document,
        "m08:character:author:01",
        vec![
            Command::PutMesh {
                mesh: authored_mesh.clone(),
            },
            Command::PutMaterial {
                material: Material::diffuse(Id(8202), [0.12, 0.45, 0.7]),
            },
            Command::CreateEntity {
                entity: Entity {
                    id: Id(8200),
                    name: "character rig".into(),
                    parent: None,
                    mesh: None,
                    material: None,
                    transform: Transform::default(),
                },
            },
            Command::CreateEntity {
                entity: Entity {
                    id: Id(8201),
                    name: "skinned block character".into(),
                    parent: None,
                    mesh: Some(authored_mesh.content_id()?),
                    material: Some(Id(8202)),
                    transform: Transform::default(),
                },
            },
            Command::CreateEntity {
                entity: Entity {
                    id: Id(8500),
                    name: "animated hand goal".into(),
                    parent: None,
                    mesh: None,
                    material: None,
                    transform: Transform::translation(1.1, 0.95, 0.),
                },
            },
            Command::ModelMesh {
                entity: Id(8201),
                source_mesh: authored_mesh.content_id()?,
                operation: modeling::Operation::Triangulate,
                budget: modeling::Budget::default(),
            },
            Command::SetAnimation {
                animation: Some(animation),
            },
            Command::SetRenderSettings { settings },
        ],
    )?;
    document.execute(&fixtures::principal(), &req)?;
    Ok(document)
}

pub fn imaging_document() -> Result<Document> {
    use crate::{geometry::*, products::*};
    use std::collections::BTreeSet;
    let mut document = Document::new(Snapshot::empty(Id(7800)))?;
    let mut commands = vec![];
    for i in 0..2 {
        let x = if i == 0 { -1. } else { 0.1 };
        let mut mesh = Mesh::from_polygons(
            Positions::F64(vec![
                [x, -0.5, 0.],
                [x + 0.9, -0.5, 0.],
                [x + 0.9, 0.5, 0.],
                [x, 0.5, 0.],
            ]),
            &[vec![0, 1, 2, 3]],
            &[],
        )?;
        mesh.attributes.insert(
            "uv".into(),
            Attribute {
                id: Id(1),
                domain: Domain::Corner,
                semantic: "uv".into(),
                transfer: Transfer::Linear,
                values: AttributeValues::Vec2(vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]]),
            },
        );
        let material = Material::diffuse(
            Id(7900 + i),
            if i == 0 {
                [0.7, 0.2, 0.05]
            } else {
                [0.1, 0.3, 0.7]
            },
        );
        commands.extend([
            Command::PutMesh { mesh: mesh.clone() },
            Command::PutMaterial { material },
            Command::CreateEntity {
                entity: Entity {
                    id: Id(7850 + i),
                    name: format!("imaging panel {i}"),
                    parent: None,
                    mesh: Some(mesh.content_id()?),
                    material: Some(Id(7900 + i)),
                    transform: Transform::default(),
                },
            },
        ]);
    }
    let mut settings = fixtures::settings();
    settings.width = 64;
    settings.height = 48;
    settings.samples = 8;
    settings.max_depth = 1;
    settings.environment = [0.3; 3];
    settings.camera = Camera {
        position: [0., 0., 2.8],
        target: [0.; 3],
        up: [0., 1., 0.],
        vertical_fov_radians: 0.6,
        lens: None,
    };
    let imaging = State {
        pipeline: Pipeline {
            destination: ColorSpace::Srgb,
            exposure_stops: 0.,
            tone: Tone::ReinhardLuminance,
            gamut: Gamut::Clip,
            denoiser: Some(Bilateral {
                radius: 2,
                spatial_sigma: 1.5,
                luminance_sigma: 0.2,
                normal_sigma: 0.2,
                relative_depth_sigma: 0.1,
            }),
        },
        views: vec![
            View {
                name: "beauty".into(),
                include: None,
            },
            View {
                name: "orange-only".into(),
                include: Some(BTreeSet::from([Id(7850)])),
            },
        ],
        bakes: vec![
            Bake {
                name: "orange-albedo".into(),
                entity: Id(7850),
                width: 32,
                height: 32,
                pass: BakePass::Albedo,
            },
            Bake {
                name: "orange-normal".into(),
                entity: Id(7850),
                width: 32,
                height: 32,
                pass: BakePass::WorldNormal,
            },
        ],
    };
    commands.extend([
        Command::SetImaging {
            imaging: Some(imaging),
        },
        Command::SetRenderSettings { settings },
    ]);
    let request = fixtures::request(&document, "m07:imaging:author:01", commands)?;
    document.execute(&fixtures::principal(), &request)?;
    Ok(document)
}

pub fn surface_document() -> Result<Document> {
    use crate::{
        displacement::*,
        geometry::*,
        scattering::{Model, Opacity, Surface},
    };
    let mut document = Document::new(Snapshot::empty(Id(8600)))?;
    let mut commands = vec![];
    let models = [
        Model::Dielectric { ior: 1.5 },
        Model::Conductor {
            eta: [0.2, 0.9, 1.1],
            k: [3., 2., 1.5],
        },
        Model::Coated {
            weight: 1.,
            ior: 1.5,
            roughness: 0.15,
        },
    ];
    for (i, model) in models.into_iter().enumerate() {
        let asset = curves::Asset {
            shape: Shape::Points {
                points: vec![Point {
                    id: Id(1),
                    position: [(i as f64 - 1.) * 0.95, 0.35, 0.],
                    radius: 0.38,
                }],
            },
            tessellation: Tessellation {
                radial_error: 0.004,
                ..Default::default()
            },
        };
        let key = asset.content_id()?;
        let mut material = Material::diffuse(
            Id(8610 + i as u128),
            if i == 2 { [0.7, 0.14, 0.03] } else { [1.; 3] },
        );
        material.roughness = if i == 0 { 0. } else { 0.3 };
        material.pbr = Some(pbr::Surface {
            advanced: Some(Surface {
                model,
                opacity: Opacity::Opaque,
            }),
            ..Default::default()
        });
        commands.extend([
            Command::PutGeometry { asset },
            Command::PutMaterial { material },
            Command::CreateEntity {
                entity: Entity {
                    id: Id(8620 + i as u128),
                    name: format!("extended surface {i}"),
                    parent: None,
                    mesh: None,
                    material: Some(Id(8610 + i as u128)),
                    transform: Transform::default(),
                },
            },
            Command::SetGeometry {
                entity: Id(8620 + i as u128),
                asset: Some(key),
            },
        ]);
    }
    let mut mesh = Mesh::from_polygons(
        Positions::F64(vec![
            [-2., -0.12, -1.2],
            [2., -0.12, -1.2],
            [2., -0.12, 1.2],
            [-2., -0.12, 1.2],
        ]),
        &[vec![0, 3, 2, 1]],
        &[],
    )?;
    let uv = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    mesh.attributes.insert(
        "uv".into(),
        Attribute {
            id: Id(1),
            domain: Domain::Corner,
            semantic: "uv".into(),
            transfer: Transfer::Linear,
            values: AttributeValues::Vec2(
                mesh.corners.iter().map(|c| uv[c.vertex as usize]).collect(),
            ),
        },
    );
    let mut material = Material::diffuse(Id(8630), [0.15, 0.3, 0.2]);
    material.pbr = Some(pbr::Surface {
        displacement: Some(Displacement {
            height: Height::Wave {
                amplitude_meters: 0.06,
                frequency: [3., 1.],
                phase_radians: 0.,
            },
            subdivisions: 4,
            max_vertices: 65536,
        }),
        ..Default::default()
    });
    commands.extend([
        Command::PutMesh { mesh: mesh.clone() },
        Command::PutMaterial { material },
        Command::CreateEntity {
            entity: Entity {
                id: Id(8631),
                name: "geometrically displaced floor".into(),
                parent: None,
                mesh: Some(mesh.content_id()?),
                material: Some(Id(8630)),
                transform: Transform::default(),
            },
        },
    ]);
    let mut settings = fixtures::settings();
    settings.width = 96;
    settings.height = 64;
    settings.samples = 16;
    settings.max_depth = 8;
    settings.environment = [0.35; 3];
    settings.camera = Camera {
        position: [1.5, 1.4, 3.5],
        target: [0., 0.2, 0.],
        up: [0., 1., 0.],
        vertical_fov_radians: 0.65,
        lens: None,
    };
    settings.light = PointLight {
        position: [-2., 4., 2.],
        intensity: [35.; 3],
    };
    commands.push(Command::SetRenderSettings { settings });
    let req = fixtures::request(&document, "m07:surfaces:author:01", commands)?;
    document.execute(&fixtures::principal(), &req)?;
    Ok(document)
}

pub fn modeling_document() -> Result<Document> {
    use crate::{modeling::*, procedural::*};
    use std::collections::BTreeMap;
    let mut document = Document::new(Snapshot::empty(Id(8700)))?;
    let group = Group {
        nodes: BTreeMap::from([
            (Id(10), Node::InputGeometry),
            (
                Id(11),
                Node::Modify {
                    geometry: Id(10),
                    operation: Operation::Array {
                        count: 3,
                        step: [0.8, 0., 0.],
                    },
                },
            ),
        ]),
        output: Id(11),
    };
    let graph = Graph {
        root: Group {
            nodes: BTreeMap::from([
                (
                    Id(1),
                    Node::Box {
                        min: [-0.3; 3],
                        max: [0.3; 3],
                    },
                ),
                (
                    Id(2),
                    Node::Modify {
                        geometry: Id(1),
                        operation: Operation::BevelBox {
                            distance_meters: 0.06,
                        },
                    },
                ),
                (
                    Id(3),
                    Node::Group {
                        geometry: Id(2),
                        group: Id(100),
                    },
                ),
                (
                    Id(4),
                    Node::Position {
                        domain: geometry::Domain::Point,
                    },
                ),
                (
                    Id(5),
                    Node::Vector {
                        value: [-0.8, 0.2, 0.],
                    },
                ),
                (Id(6), Node::AddVector { a: Id(4), b: Id(5) }),
                (
                    Id(7),
                    Node::SetPositions {
                        geometry: Id(3),
                        positions: Id(6),
                    },
                ),
            ]),
            output: Id(7),
        },
        groups: BTreeMap::from([(Id(100), group)]),
        budget: Budget::default(),
    };
    let mut material = Material::diffuse(Id(8710), [0.15, 0.4, 0.7]);
    material.pbr = Some(pbr::Surface::default());
    material.roughness = 0.5;
    let mut settings = fixtures::settings();
    settings.width = 96;
    settings.height = 64;
    settings.samples = 8;
    settings.max_depth = 1;
    settings.environment = [0.4; 3];
    settings.camera = Camera {
        position: [1.5, 1.2, 3.],
        target: [0., 0.2, 0.],
        up: [0., 1., 0.],
        vertical_fov_radians: 0.6,
        lens: None,
    };
    let req = fixtures::request(
        &document,
        "m06:procedural:author:01",
        vec![
            Command::PutMaterial { material },
            Command::CreateEntity {
                entity: Entity {
                    id: Id(8720),
                    name: "bevel array with a point field".into(),
                    parent: None,
                    mesh: None,
                    material: Some(Id(8710)),
                    transform: Transform::default(),
                },
            },
            Command::SetProcedural {
                entity: Id(8720),
                graph: Some(graph),
            },
            Command::SetRenderSettings { settings },
        ],
    )?;
    document.execute(&fixtures::principal(), &req)?;
    Ok(document)
}
