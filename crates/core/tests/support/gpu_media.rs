use render_core::{document::*, render::*, *};
pub struct Case {
    pub name: String,
    pub document: Document,
    pub settings: Settings,
    pub expected: [f32; 3],
}
pub fn cases() -> Vec<Case> {
    let data: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../fixtures/gpu-media/data/cases.json"
    ))
    .unwrap();
    data["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            let mut document = Document::new(Snapshot::empty(Id(10600))).unwrap();
            let mut settings = fixtures::settings();
            settings.width = 1;
            settings.height = 1;
            settings.samples = 1;
            settings.max_depth = 2;
            settings.environment = serde_json::from_value(data["environment"].clone()).unwrap();
            settings.light.intensity = [0.; 3];
            settings.camera.position =
                serde_json::from_value(row["camera"]["position"].clone()).unwrap();
            settings.camera.target =
                serde_json::from_value(row["camera"]["target"].clone()).unwrap();
            settings.camera.lens = Some(cameras::Lens::Orthographic {
                xmag: 1e-7,
                ymag: 1e-7,
                near: row["camera"]["near"].as_f64().unwrap(),
                far: row["camera"]["far"].as_f64().unwrap(),
            });
            let mut commands = vec![];
            for (i, m) in row["media"].as_array().unwrap().iter().enumerate() {
                let asset: volumes::Asset = serde_json::from_value(m["asset"].clone()).unwrap();
                let id = Id(10601 + i as u128);
                commands.extend([
                    Command::CreateEntity {
                        entity: Entity {
                            id,
                            name: format!("medium {i}"),
                            parent: None,
                            mesh: None,
                            material: None,
                            transform: serde_json::from_value(m["transform"].clone()).unwrap(),
                        },
                    },
                    Command::SetVolume {
                        entity: id,
                        asset: Some(asset.content_id().unwrap()),
                    },
                    Command::PutVolume { asset },
                ]);
            }
            commands.push(Command::SetRenderSettings {
                settings: settings.clone(),
            });
            document
                .execute(
                    &fixtures::principal(),
                    &fixtures::request(&document, "media:analytic:author:001", commands).unwrap(),
                )
                .unwrap();
            Case {
                name: row["name"].as_str().unwrap().into(),
                document,
                settings,
                expected: serde_json::from_value(row["expected"].clone()).unwrap(),
            }
        })
        .collect()
}
