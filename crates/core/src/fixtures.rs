use crate::{Id, Result, document::*, geometry::*, render::*};
use std::collections::BTreeMap;

pub fn seam_mesh() -> Result<Mesh> {
    let mut m = Mesh::from_polygons(
        Positions::F32(vec![
            [-1., 0., -1.],
            [1., 0., -1.],
            [1., 0., 1.],
            [-1., 0., 1.],
            [0., 1., 0.],
            [3., 0., 0.],
            [4., 0., 0.],
            [5., 0., 0.],
        ]),
        &[
            vec![0, 3, 2, 1],
            vec![0, 1, 4],
            vec![1, 2, 4],
            vec![2, 3, 4],
            vec![3, 0, 4],
        ],
        &[[5, 6]],
    )?;
    let uv = m
        .corners
        .iter()
        .enumerate()
        .map(|(i, c)| [c.vertex as f32 * 0.25, (i % 3) as f32 * 0.5])
        .collect();
    m.attributes.insert(
        "UVMap".into(),
        Attribute {
            id: Id(20),
            domain: Domain::Corner,
            semantic: "uv".into(),
            transfer: Transfer::Linear,
            values: AttributeValues::Vec2(uv),
        },
    );
    m.validate()?;
    Ok(m)
}
pub fn nonmanifold() -> Result<Mesh> {
    Mesh::from_polygons(
        Positions::F32(vec![
            [0., 0., 0.],
            [1., 0., 0.],
            [0., 1., 0.],
            [0., 0., 1.],
            [0., -1., 0.],
        ]),
        &[vec![0, 1, 2], vec![1, 0, 3], vec![0, 1, 4]],
        &[],
    )
}
pub fn principal() -> Principal {
    Principal {
        id: "local-fixture".into(),
        can_write: true,
    }
}
/// Analytic Lambertian plane: every camera ray hits, every scattered ray escapes.
/// With no point-light contribution, radiance must equal albedo * environment.
pub fn diffuse_plane() -> Result<(Scene, Settings, [f32; 3])> {
    let mesh = Mesh::from_polygons(
        Positions::F32(vec![
            [-100., -100., 0.],
            [100., -100., 0.],
            [100., 100., 0.],
            [-100., 100., 0.],
        ]),
        &[vec![0, 1, 2, 3]],
        &[],
    )?;
    let key = mesh.content_id()?;
    let mut doc = Document::new(Snapshot::empty(Id(900)))?;
    let request = request(
        &doc,
        "analytic:plane:001",
        vec![
            Command::PutMesh { mesh },
            Command::PutMaterial {
                material: Material::diffuse(Id(901), [0.5, 0.25, 0.75]),
            },
            Command::CreateEntity {
                entity: Entity {
                    id: Id(902),
                    name: "analytic plane".into(),
                    parent: None,
                    mesh: Some(key),
                    material: Some(Id(901)),
                    transform: Transform::default(),
                },
            },
        ],
    )?;
    doc.execute(&principal(), &request)?;
    let mut settings = settings();
    settings.width = 16;
    settings.height = 12;
    settings.camera.position = [0., 0., 2.];
    settings.camera.target = [0., 0., 0.];
    settings.camera.up = [0., 1., 0.];
    settings.light.intensity = [0.; 3];
    settings.environment = [0.2, 0.4, 0.6];
    Ok((
        Evaluator::default().evaluate(doc.snapshot())?,
        settings,
        [0.1, 0.1, 0.45],
    ))
}
pub fn request(doc: &Document, key: &str, commands: Vec<Command>) -> Result<Request> {
    Ok(Request {
        version: 0,
        base_revision: doc.snapshot().revision()?,
        idempotency_key: key.into(),
        commands,
        max_added_bytes: 64 * 1024 * 1024,
    })
}
pub fn demo() -> Result<Document> {
    let mut doc = Document::new(Snapshot::empty(Id(1)))?;
    let mesh = seam_mesh()?;
    let key = mesh.content_id()?;
    let ground = Mesh::from_polygons(
        Positions::F32(vec![
            [-8., -0.02, -8.],
            [-8., -0.02, 8.],
            [8., -0.02, 8.],
            [8., -0.02, -8.],
        ]),
        &[vec![0, 1, 2, 3]],
        &[],
    )?;
    let ground_key = ground.content_id()?;
    let mut material = Material::diffuse(Id(2), [0.75, 0.28, 0.12]);
    material.texture = Some(Texture {
        width: 2,
        height: 2,
        linear_rgb: vec![[1.; 3], [0.5; 3], [0.5; 3], [1.; 3]],
    });
    let commands = vec![
        Command::PutMesh { mesh },
        Command::PutMesh { mesh: ground },
        Command::PutMaterial { material },
        Command::PutMaterial {
            material: Material::diffuse(Id(3), [0.5, 0.55, 0.6]),
        },
        Command::CreateEntity {
            entity: Entity {
                id: Id(4),
                name: "Pyramid A".into(),
                parent: None,
                mesh: Some(key.clone()),
                material: Some(Id(2)),
                transform: Transform::translation(-1.3, 0., 0.),
            },
        },
        Command::CreateEntity {
            entity: Entity {
                id: Id(5),
                name: "Pyramid B".into(),
                parent: None,
                mesh: Some(key),
                material: Some(Id(2)),
                transform: Transform::translation(1.3, 0., 0.),
            },
        },
        Command::CreateEntity {
            entity: Entity {
                id: Id(6),
                name: "Ground".into(),
                parent: None,
                mesh: Some(ground_key),
                material: Some(Id(3)),
                transform: Transform::default(),
            },
        },
    ];
    let r = request(&doc, "demo:create:00001", commands)?;
    doc.execute(&principal(), &r)?;
    Ok(doc)
}
pub fn settings() -> Settings {
    Settings {
        width: 96,
        height: 64,
        samples: 8,
        seed: 42,
        max_depth: 1,
        max_bytes: 64 * 1024 * 1024,
        environment: [0.12, 0.15, 0.2],
        light: PointLight {
            position: [-2., 5., 3.],
            intensity: [100., 90., 75.],
        },
        camera: Camera {
            lens: None,
            position: [5., 3., 6.],
            target: [0., 0.4, 0.],
            up: [0., 1., 0.],
            vertical_fov_radians: 0.7,
        },
    }
}
pub fn conformance() -> Result<BTreeMap<String, bool>> {
    let mut report = BTreeMap::new();
    let d = demo()?;
    let bytes = crate::canonical(&d)?;
    let loaded: Document = serde_json::from_slice(&bytes)?;
    report.insert(
        "document_roundtrip".into(),
        d.snapshot().revision()? == loaded.snapshot().revision()?,
    );
    let mut branch = d.clone();
    let r = request(
        &branch,
        "fixture:rename:01",
        vec![Command::Rename {
            entity: Id(4),
            name: "Renamed".into(),
        }],
    )?;
    branch.execute(&principal(), &r)?;
    report.insert(
        "branch_isolation".into(),
        d.snapshot().entities.get(Id(4)).expect("fixture").name == "Pyramid A",
    );
    report.insert(
        "idempotent_retry".into(),
        branch.execute(&principal(), &r)?.revision == branch.snapshot().revision()?,
    );
    let mut evaluator = Evaluator::default();
    let a = evaluator.evaluate(d.snapshot())?;
    let b = evaluator.evaluate(branch.snapshot())?;
    report.insert(
        "geometry_cache".into(),
        a.geometry_builds == 2 && b.geometry_builds == 0,
    );
    report.insert(
        "nonmanifold".into(),
        !crate::topology::EditMesh::new(&nonmanifold()?)?.manifold_candidate_accepts(),
    );
    if report.values().any(|v| !*v) {
        return Err(crate::Error::new(
            "conformance",
            "shared-core conformance failed",
        ));
    }
    Ok(report)
}
