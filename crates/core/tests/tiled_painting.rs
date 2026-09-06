use render_core::{Id, Time, canonical, painting::*, textures::TextureRole};
use std::{collections::BTreeMap, sync::Arc};
fn canvas(width: u32, height: u32) -> Asset {
    Asset {
        version: 0,
        width,
        height,
        role: TextureRole::LinearData,
        layers: vec![Layer::empty(Id(1), "Base".into())],
        strokes: vec![],
    }
}
fn sample(x: f64, y: f64, t: i64) -> Sample {
    Sample {
        pixel: [x, y],
        pressure: 1.,
        time: Time::new(t, 10).unwrap(),
    }
}
fn brush(mode: Mode) -> Brush {
    Brush {
        radius: 2.,
        hardness: 1.,
        spacing: 1.,
        mode,
    }
}
fn input(id: u128, mode: Mode, samples: Vec<Sample>) -> StrokeInput {
    StrokeInput {
        id: Id(id),
        layer: Id(1),
        brush: brush(mode),
        samples,
        finish: true,
    }
}
fn paint_color(color: [f64; 4]) -> Mode {
    Mode::Paint { color }
}
fn perform(
    asset: &mut Asset,
    tiles: &mut BTreeMap<String, Arc<Tile>>,
    q: &StrokeInput,
) -> StrokeReport {
    let out = paint(asset, tiles, q, &Budget::default(), || false).unwrap();
    tiles.extend(out.tiles);
    *asset = out.asset;
    asset.validate_tiles(tiles).unwrap();
    out.report
}
fn pixel(asset: &Asset, tiles: &BTreeMap<String, Arc<Tile>>, x: u32, y: u32) -> [u8; 4] {
    let result = bake(asset, tiles, &BakeBudget::default(), || false).unwrap();
    image::load_from_memory(&result.image.encoded)
        .unwrap()
        .to_rgba8()
        .get_pixel(x, y)
        .0
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn tile_encoding_is_explicit_and_rejects_invalid_payloads() {
    let mut tile = Tile::transparent();
    let Tile::Color { rgba_le, .. } = &mut tile else {
        unreachable!()
    };
    rgba_le[0] = [1, 256, 513, 65535];
    let value = serde_json::to_value(&tile).unwrap();
    assert!(
        value["rgba_le"]
            .as_str()
            .unwrap()
            .starts_with("010000010102ffff")
    );
    assert_eq!(serde_json::from_value::<Tile>(value.clone()).unwrap(), tile);
    for bad in ["00", &"A".repeat(16384), &"0".repeat(16385)] {
        let mut v = value.clone();
        v["rgba_le"] = bad.into();
        assert!(serde_json::from_value::<Tile>(v).is_err());
    }
    let mut v = value.clone();
    v["extra"] = true.into();
    assert!(serde_json::from_value::<Tile>(v).is_err());
    let mut invalid = tile.clone();
    let Tile::Color { rgba_le, .. } = &mut invalid else {
        unreachable!()
    };
    rgba_le[0][3] = 0;
    assert_eq!(invalid.content_id().unwrap_err().code, "paint_tile");
    let r = TileRef {
        coordinate: Coordinate { x: 0, y: 0 },
        tile: tile.content_id().unwrap(),
    };
    assert!(
        serde_json::from_value::<TileMap>(serde_json::to_value(vec![r.clone(), r]).unwrap())
            .is_err()
    );
    let mut mask = Tile::full_mask();
    let Tile::Mask { coverage_le, .. } = &mut mask else {
        unreachable!()
    };
    coverage_le[0] = 513;
    let encoded = serde_json::to_value(&mask).unwrap();
    assert!(
        encoded["coverage_le"]
            .as_str()
            .unwrap()
            .starts_with("0102ffff")
    );
    assert_eq!(serde_json::from_value::<Tile>(encoded).unwrap(), mask);
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn edge_padding_references_layer_ids_and_caps_are_validated() {
    let mut asset = canvas(1, 1);
    let mut tile = Tile::transparent();
    let Tile::Color { rgba_le, .. } = &mut tile else {
        unreachable!()
    };
    rgba_le[1] = [65535; 4];
    let hash = tile.content_id().unwrap();
    let mut tiles = BTreeMap::from([(hash.clone(), Arc::new(tile))]);
    asset.layers[0]
        .color
        .0
        .insert(Coordinate { x: 0, y: 0 }, hash);
    assert_eq!(asset.validate_tiles(&tiles).unwrap_err().code, "paint_tile");
    asset.layers[0].color.0.clear();
    let mask = Tile::full_mask();
    let hash = mask.content_id().unwrap();
    tiles.insert(hash.clone(), Arc::new(mask));
    asset.layers[0]
        .color
        .0
        .insert(Coordinate { x: 0, y: 0 }, hash);
    assert_eq!(asset.validate_tiles(&tiles).unwrap_err().code, "paint_tile");
    asset.layers[0].color.0.clear();
    asset.layers.push(asset.layers[0].clone());
    assert!(asset.validate().is_err());
    let mut asset = canvas(2048, 2048);
    asset.layers[0]
        .color
        .0
        .insert(Coordinate { x: u32::MAX, y: 0 }, "x".into());
    assert!(asset.validate().is_err());
    assert!(canvas(2049, 1).validate().is_err());
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn source_over_erase_and_mask_match_independent_alpha_equations() {
    // UNORM16 half is 32768. After over: [32767,0,32768,65535].
    // Erase factor 32767 yields [16383,0,16384,32767]; straight red
    // rounds to 127. Mask factor 32767 yields [8191,0,8192,16383].
    let mut asset = canvas(32, 32);
    let mut tiles = BTreeMap::new();
    perform(
        &mut asset,
        &mut tiles,
        &input(1, paint_color([1., 0., 0., 1.]), vec![sample(8.5, 8.5, 0)]),
    );
    perform(
        &mut asset,
        &mut tiles,
        &input(2, paint_color([0., 0., 1., 0.5]), vec![sample(8.5, 8.5, 0)]),
    );
    assert_eq!(pixel(&asset, &tiles, 8, 8), [127, 0, 128, 255]);
    perform(
        &mut asset,
        &mut tiles,
        &input(3, Mode::Erase { opacity: 0.5 }, vec![sample(8.5, 8.5, 0)]),
    );
    assert_eq!(pixel(&asset, &tiles, 8, 8), [127, 0, 128, 127]);
    let color = asset.layers[0].color.clone();
    perform(
        &mut asset,
        &mut tiles,
        &input(
            4,
            Mode::Mask {
                value: 0.,
                opacity: 0.5,
            },
            vec![sample(8.5, 8.5, 0)],
        ),
    );
    assert_eq!(asset.layers[0].color, color);
    assert_eq!(pixel(&asset, &tiles, 8, 8), [127, 0, 128, 64]);
    perform(
        &mut asset,
        &mut tiles,
        &input(
            5,
            Mode::Mask {
                value: 1.,
                opacity: 1.,
            },
            vec![sample(8.5, 8.5, 0)],
        ),
    );
    assert_eq!(pixel(&asset, &tiles, 8, 8), [127, 0, 128, 127]);
    perform(
        &mut asset,
        &mut tiles,
        &input(6, Mode::Erase { opacity: 1. }, vec![sample(8.5, 8.5, 0)]),
    );
    assert_eq!(pixel(&asset, &tiles, 8, 8), [0; 4]);
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn layer_order_visibility_and_opacity_affect_real_composite() {
    let mut asset = canvas(16, 16);
    let mut tiles = BTreeMap::new();
    perform(
        &mut asset,
        &mut tiles,
        &input(1, paint_color([1., 0., 0., 1.]), vec![sample(8.5, 8.5, 0)]),
    );
    asset.layers.push(Layer::empty(Id(2), "Top".into()));
    let mut q = input(2, paint_color([0., 1., 0., 1.]), vec![sample(8.5, 8.5, 0)]);
    q.layer = Id(2);
    perform(&mut asset, &mut tiles, &q);
    assert_eq!(pixel(&asset, &tiles, 8, 8), [0, 255, 0, 255]);
    asset.layers[1].opacity = 32768;
    assert_eq!(pixel(&asset, &tiles, 8, 8), [127, 128, 0, 255]);
    asset.layers[1].visible = false;
    assert_eq!(pixel(&asset, &tiles, 8, 8), [255, 0, 0, 255]);
    asset.layers[1].visible = true;
    asset.layers.reverse();
    assert_eq!(pixel(&asset, &tiles, 8, 8), [255, 0, 0, 255]);
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn stroke_checkpoint_replay_matches_one_shot_and_preserves_distant_tiles() {
    let mut asset = canvas(128, 64);
    let mut tiles = BTreeMap::new();
    perform(
        &mut asset,
        &mut tiles,
        &input(1, paint_color([0., 1., 0., 1.]), vec![sample(110., 10., 0)]),
    );
    let distant = asset.layers[0].color.0[&Coordinate { x: 3, y: 0 }].clone();
    let original = canonical(&asset).unwrap();
    let samples = vec![
        sample(28.5, 10., 0),
        sample(35.2, 14., 1),
        sample(35.2, 14., 2),
        sample(57.7, 19., 3),
    ];
    let mut q = input(2, paint_color([1., 0.3, 0.2, 0.3]), samples.clone());
    q.brush.spacing = 0.37;
    q.brush.hardness = 0.3;
    q.samples[1].pressure = 0.4;
    q.samples[2].pressure = 0.8;
    let full = paint(&asset, &tiles, &q, &Budget::default(), || false).unwrap();
    let mut chunk = q.clone();
    chunk.samples = q.samples[..2].to_vec();
    chunk.finish = false;
    let mut partial = asset.clone();
    let mut partial_tiles = tiles.clone();
    perform(&mut partial, &mut partial_tiles, &chunk);
    // Actual persistence round trip between chunks, no private accumulator required.
    partial = serde_json::from_slice(&canonical(&partial).unwrap()).unwrap();
    partial_tiles = serde_json::from_slice(&canonical(&partial_tiles).unwrap()).unwrap();
    chunk.samples = q.samples[2..].to_vec();
    chunk.finish = true;
    let report = perform(&mut partial, &mut partial_tiles, &chunk);
    assert_eq!(partial, full.asset);
    assert_eq!(canonical(&asset).unwrap(), original);
    assert_eq!(
        partial.layers[0].color.0[&Coordinate { x: 3, y: 0 }],
        distant
    );
    assert!(report.touched_tiles.iter().all(|c| c.x < 2));
    assert_eq!(
        report.copied_tile_bytes,
        8192 * report.touched_tiles.len() as u64
    );
    for (hash, tile) in full.tiles {
        assert_eq!(partial_tiles[&hash], tile);
    }
    assert_eq!(
        paint(&partial, &partial_tiles, &chunk, &Budget::default(), || {
            false
        })
        .err()
        .unwrap()
        .code,
        "paint_stroke"
    );
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn brush_coverage_clipping_pressure_and_work_are_bounded() {
    let asset = canvas(32, 32);
    let tiles = BTreeMap::new();
    let mut q = input(1, paint_color([1.; 4]), vec![sample(0.5, 0.5, 0)]);
    q.brush.radius = 0.25;
    let none = paint(&asset, &tiles, &q, &Budget::default(), || false).unwrap();
    assert!(none.tiles.is_empty());
    q.brush.radius = 0.5;
    let full = paint(&asset, &tiles, &q, &Budget::default(), || false).unwrap();
    let tile = full.tiles.values().next().unwrap();
    let Tile::Color { rgba_le, .. } = tile.as_ref() else {
        unreachable!()
    };
    assert_eq!(rgba_le[0], [65535; 4]);
    assert_eq!(rgba_le[1], [0; 4]);
    q.samples[0].pressure = 0.5;
    let half = paint(&asset, &tiles, &q, &Budget::default(), || false).unwrap();
    let Tile::Color { rgba_le, .. } = half.tiles.values().next().unwrap().as_ref() else {
        unreachable!()
    };
    assert_eq!(rgba_le[0], [32768; 4]);
    q.samples = vec![sample(-100., -100., 0)];
    let out = paint(&asset, &tiles, &q, &Budget::default(), || false).unwrap();
    assert_eq!(out.report.emitted_dabs, 1);
    assert_eq!(out.report.pixel_work, 0);
    q.samples = vec![sample(1., 1., 1), sample(5., 1., 0)];
    assert_eq!(
        paint(&asset, &tiles, &q, &Budget::default(), || false)
            .err()
            .unwrap()
            .code,
        "paint_sample"
    );
    q.samples = vec![sample(8., 8., 0)];
    q.brush.radius = 4.;
    for budget in [
        Budget {
            max_pixel_work: 1,
            ..Budget::default()
        },
        Budget {
            max_output_bytes: 1,
            ..Budget::default()
        },
    ] {
        assert_eq!(
            paint(&asset, &tiles, &q, &budget, || false)
                .err()
                .unwrap()
                .code,
            "budget"
        );
    }
    q.samples = vec![sample(1., 1., 0), sample(100., 1., 1)];
    assert_eq!(
        paint(
            &asset,
            &tiles,
            &q,
            &Budget {
                max_dabs: 1,
                ..Budget::default()
            },
            || false
        )
        .err()
        .unwrap()
        .code,
        "budget"
    );
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn cancellation_at_every_observed_checkpoint_keeps_sources_immutable() {
    let asset = canvas(64, 32);
    let tiles = BTreeMap::new();
    let before = canonical(&asset).unwrap();
    let q = input(
        1,
        paint_color([1., 0., 0., 0.5]),
        vec![sample(30., 16., 0), sample(34., 16., 1)],
    );
    let mut checks = 0;
    let out = paint(&asset, &tiles, &q, &Budget::default(), || {
        checks += 1;
        false
    })
    .unwrap();
    for stop in 1..=checks {
        let mut n = 0;
        assert_eq!(
            paint(&asset, &tiles, &q, &Budget::default(), || {
                n += 1;
                n == stop
            })
            .err()
            .unwrap()
            .code,
            "cancelled"
        );
    }
    assert_eq!(canonical(&asset).unwrap(), before);
    let mut checks = 0;
    bake(&out.asset, &out.tiles, &BakeBudget::default(), || {
        checks += 1;
        false
    })
    .unwrap();
    for stop in 1..=checks {
        let mut n = 0;
        assert_eq!(
            bake(&out.asset, &out.tiles, &BakeBudget::default(), || {
                n += 1;
                n == stop
            })
            .err()
            .unwrap()
            .code,
            "cancelled"
        );
    }
    assert_eq!(
        bake(
            &out.asset,
            &out.tiles,
            &BakeBudget {
                max_pixel_work: 1,
                ..BakeBudget::default()
            },
            || false
        )
        .err()
        .unwrap()
        .code,
        "budget"
    );
    assert_eq!(
        bake(
            &out.asset,
            &out.tiles,
            &BakeBudget {
                max_encoded_bytes: 1,
                ..BakeBudget::default()
            },
            || false
        )
        .err()
        .unwrap()
        .code,
        "budget"
    );
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn bake_uses_explicit_color_role_and_reports_premultiplied_error() {
    let mut asset = canvas(1, 1);
    let mut tile = Tile::transparent();
    let Tile::Color { rgba_le, .. } = &mut tile else {
        unreachable!()
    };
    rgba_le[0] = [32768, 16384, 0, 65535];
    let hash = tile.content_id().unwrap();
    let tiles = BTreeMap::from([(hash.clone(), Arc::new(tile))]);
    asset.layers[0]
        .color
        .0
        .insert(Coordinate { x: 0, y: 0 }, hash);
    assert_eq!(pixel(&asset, &tiles, 0, 0), [128, 64, 0, 255]);
    asset.role = TextureRole::SrgbColor;
    assert_eq!(pixel(&asset, &tiles, 0, 0), [188, 137, 0, 255]);
    let out = bake(&asset, &tiles, &BakeBudget::default(), || false).unwrap();
    assert!(out.report.max_premultiplied_quantization_error > 0.);
    assert!(out.report.max_premultiplied_quantization_error < 0.004);
    assert_eq!(out.report.rgba8_bytes, 4);
    assert_eq!(out.report.pixel_work, 1);
}
fn paint_document() -> (render_core::document::Document, Request) {
    use render_core::{document::*, fixtures};
    let mut doc = fixtures::demo().unwrap();
    let asset = canvas(64, 32);
    let hash = asset.content_id().unwrap();
    let init = fixtures::request(
        &doc,
        "painting-create-canvas",
        vec![
            Command::PutPaintAsset { asset },
            Command::SetPaintCanvas {
                canvas: Id(900),
                source_asset: None,
                asset: Some(hash.clone()),
            },
            Command::SetRenderSettings {
                settings: fixtures::settings(),
            },
        ],
    )
    .unwrap();
    doc.execute(&fixtures::principal(), &init).unwrap();
    let q = render_core::painting::Request {
        version: 0,
        base_revision: doc.snapshot().revision().unwrap(),
        idempotency_key: "painting-first-stroke".into(),
        canvas: Id(900),
        source_asset: hash,
        stroke: input(
            1,
            paint_color([1., 0., 0., 0.5]),
            vec![sample(30., 16., 0), sample(34., 16., 1)],
        ),
        budget: Budget::default(),
        max_added_bytes: 4 * 1024 * 1024,
    };
    (doc, q)
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn ordinary_transactions_retry_bake_recover_and_undo_retained_canvases() {
    use render_core::{document::*, fixtures, storage::*};
    let (mut doc, q) = paint_document();
    let p = fixtures::principal();
    let before = canonical(doc.snapshot()).unwrap();
    let materials = doc.snapshot().materials.clone();
    let prepared = prepare(&doc, &p, &q, || false).unwrap();
    let commands = canonical(&prepared.transaction).unwrap();
    let mut store = MemoryStore::default();
    let receipt = durable_execute(&mut store, &mut doc, &p, &prepared.transaction).unwrap();
    assert!(receipt.durable);
    assert_eq!(doc.snapshot().version, 18);
    assert_eq!(doc.snapshot().materials, materials);
    assert_eq!(
        doc.snapshot().paint_assets[&q.source_asset].layers[0]
            .color
            .0
            .len(),
        0
    );
    let retry = prepare(&doc, &p, &q, || false).unwrap();
    assert_eq!(canonical(&retry.transaction).unwrap(), commands);
    assert_eq!(
        canonical(&durable_execute(&mut store, &mut doc, &p, &retry.transaction).unwrap()).unwrap(),
        canonical(&receipt).unwrap()
    );
    assert_eq!(store.records.len(), 1);
    let bake_q = BakeRequest {
        version: 0,
        base_revision: doc.snapshot().revision().unwrap(),
        idempotency_key: "painting-explicit-bake".into(),
        canvas: q.canvas,
        source_asset: prepared.report.output.clone(),
        budget: BakeBudget::default(),
        max_added_bytes: 4 * 1024 * 1024,
    };
    let baked = prepare_bake(&doc, &p, &bake_q, || false).unwrap();
    durable_execute(&mut store, &mut doc, &p, &baked.transaction).unwrap();
    assert!(doc.snapshot().images.contains_key(&baked.report.image));
    assert_eq!(doc.snapshot().materials, materials);
    let retry_bake = prepare_bake(&doc, &p, &bake_q, || false).unwrap();
    assert_eq!(
        canonical(&retry_bake.transaction).unwrap(),
        canonical(&baked.transaction).unwrap()
    );
    let restored = recover(&store.records).unwrap().unwrap();
    assert_eq!(canonical(&restored).unwrap(), canonical(&doc).unwrap());
    let undo = fixtures::request(
        &doc,
        "painting-undo-to-empty",
        vec![Command::SetPaintCanvas {
            canvas: q.canvas,
            source_asset: Some(prepared.report.output.clone()),
            asset: Some(q.source_asset.clone()),
        }],
    )
    .unwrap();
    durable_execute(&mut store, &mut doc, &p, &undo).unwrap();
    assert_eq!(doc.snapshot().paint_canvases[&q.canvas], q.source_asset);
    assert_eq!(
        canonical(&prepare(&doc, &p, &q, || false).unwrap().transaction).unwrap(),
        commands
    );
    let mut corrupt = doc.snapshot().clone();
    corrupt.version = 17;
    assert_eq!(corrupt.validate().unwrap_err().code, "schema_version");
    let old: Snapshot = serde_json::from_slice(&before).unwrap();
    assert_eq!(canonical(&old).unwrap(), before);
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn paint_publication_denials_stale_inputs_and_all_preparation_cancellation_are_atomic() {
    use render_core::{document::*, fixtures, storage::*};
    let (mut doc, q) = paint_document();
    let p = fixtures::principal();
    let before = canonical(&doc).unwrap();
    let prepared = prepare(&doc, &p, &q, || false).unwrap();
    for (quota, fail_publication, code) in [(1, false, "quota"), (0, true, "storage")] {
        let mut store = MemoryStore {
            quota,
            fail_publication,
            ..Default::default()
        };
        assert_eq!(
            durable_execute(&mut store, &mut doc, &p, &prepared.transaction)
                .unwrap_err()
                .code,
            code
        );
        assert!(store.records.is_empty());
        assert_eq!(canonical(&doc).unwrap(), before);
    }
    let denied = Principal {
        can_write: false,
        ..p.clone()
    };
    assert_eq!(
        prepare(&doc, &denied, &q, || false).err().unwrap().code,
        "permission"
    );
    let mut bad = q.clone();
    bad.base_revision = "sha256:stale".into();
    assert_eq!(
        prepare(&doc, &p, &bad, || false).err().unwrap().code,
        "stale_revision"
    );
    let mut count = 0;
    prepare(&doc, &p, &q, || {
        count += 1;
        false
    })
    .unwrap();
    for stop in 1..=count {
        let mut n = 0;
        assert_eq!(
            prepare(&doc, &p, &q, || {
                n += 1;
                n == stop
            })
            .err()
            .unwrap()
            .code,
            "cancelled"
        );
        assert_eq!(canonical(&doc).unwrap(), before);
    }
    doc.execute(&p, &prepared.transaction).unwrap();
    let mut bad = q.clone();
    bad.base_revision = doc.snapshot().revision().unwrap();
    bad.idempotency_key = "painting-stale-selection".into();
    assert_eq!(
        prepare(&doc, &p, &bad, || false).err().unwrap().code,
        "stale_selection"
    );
    let mut different = q.clone();
    different.stroke.samples[0].pressure = 0.1;
    assert_eq!(
        prepare(&doc, &p, &different, || false).err().unwrap().code,
        "idempotency_mismatch"
    );
    let bake_q = BakeRequest {
        version: 0,
        base_revision: doc.snapshot().revision().unwrap(),
        idempotency_key: "painting-bake-validation".into(),
        canvas: q.canvas,
        source_asset: prepared.report.output,
        budget: BakeBudget::default(),
        max_added_bytes: 4 * 1024 * 1024,
    };
    assert_eq!(
        prepare_bake(&doc, &denied, &bake_q, || false)
            .err()
            .unwrap()
            .code,
        "permission"
    );
    let mut bad = bake_q.clone();
    bad.base_revision = q.base_revision;
    assert_eq!(
        prepare_bake(&doc, &p, &bad, || false).err().unwrap().code,
        "stale_revision"
    );
    let mut bad = bake_q.clone();
    bad.source_asset = q.source_asset;
    assert_eq!(
        prepare_bake(&doc, &p, &bad, || false).err().unwrap().code,
        "stale_selection"
    );
    let mut count = 0;
    prepare_bake(&doc, &p, &bake_q, || {
        count += 1;
        false
    })
    .unwrap();
    for stop in [1, count / 2, count] {
        let mut n = 0;
        assert_eq!(
            prepare_bake(&doc, &p, &bake_q, || {
                n += 1;
                n == stop
            })
            .err()
            .unwrap()
            .code,
            "cancelled"
        );
    }
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn agent_paint_and_bake_use_durable_publication_and_late_cancellation() {
    use render_core::{agent, fixtures, storage::*};
    let (doc, q) = paint_document();
    let p = fixtures::principal();
    let request = agent::Request {
        version: 0,
        operation: agent::Operation::AuthorPaint { request: q.clone() },
    };
    let mut probe = agent::Session::new(doc.clone());
    let mut count = 0;
    probe
        .dispatch_cancellable(&p, request.clone(), || {
            count += 1;
            false
        })
        .unwrap();
    for stop in [1, count / 2, count] {
        let mut session = agent::Session::new(doc.clone());
        let before = canonical(session.document()).unwrap();
        let mut n = 0;
        assert_eq!(
            session
                .dispatch_cancellable(&p, request.clone(), || {
                    n += 1;
                    n == stop
                })
                .unwrap_err()
                .code,
            "cancelled"
        );
        assert_eq!(canonical(session.document()).unwrap(), before);
    }
    let mut session = agent::Session::new(doc);
    let mut store = MemoryStore {
        fail_publication: true,
        ..Default::default()
    };
    let before = canonical(session.document()).unwrap();
    assert_eq!(
        session
            .dispatch_durable(&mut store, &p, request.clone())
            .unwrap_err()
            .code,
        "storage"
    );
    assert_eq!(canonical(session.document()).unwrap(), before);
    store.fail_publication = false;
    let out = session
        .dispatch_durable(&mut store, &p, request.clone())
        .unwrap();
    assert!(out["receipt"]["durable"].as_bool().unwrap());
    session.dispatch_durable(&mut store, &p, request).unwrap();
    assert_eq!(store.records.len(), 1);
    let bake_q = BakeRequest {
        version: 0,
        base_revision: session.document().snapshot().revision().unwrap(),
        idempotency_key: "painting-agent-bake".into(),
        canvas: q.canvas,
        source_asset: out["report"]["output"].as_str().unwrap().into(),
        budget: BakeBudget::default(),
        max_added_bytes: 4 * 1024 * 1024,
    };
    let request = agent::Request {
        version: 0,
        operation: agent::Operation::BakePaint { request: bake_q },
    };
    let mut probe = session.clone();
    let mut count = 0;
    probe
        .dispatch_cancellable(&p, request.clone(), || {
            count += 1;
            false
        })
        .unwrap();
    let before = canonical(session.document()).unwrap();
    let mut n = 0;
    assert_eq!(
        session
            .dispatch_cancellable(&p, request.clone(), || {
                n += 1;
                n == count
            })
            .unwrap_err()
            .code,
        "cancelled"
    );
    assert_eq!(canonical(session.document()).unwrap(), before);
    let out = session.dispatch_durable(&mut store, &p, request).unwrap();
    assert!(out["receipt"]["durable"].as_bool().unwrap());
    assert_eq!(store.records.len(), 2);
    assert_eq!(
        canonical(&recover(&store.records).unwrap().unwrap()).unwrap(),
        canonical(session.document()).unwrap()
    );
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn hostile_parameters_strict_fields_and_retained_resource_caps_reject() {
    let mut q = input(1, paint_color([1.; 4]), vec![sample(4., 4., 0)]);
    for value in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
        q.brush.mode = Mode::Paint {
            color: [value, 0., 0., 1.],
        };
        assert!(q.brush.validate().is_err());
        q.brush.mode = Mode::Mask { value, opacity: 1. };
        assert!(q.brush.validate().is_err());
    }
    q.brush = brush(paint_color([1.; 4]));
    let mut wire = serde_json::to_value(&q).unwrap();
    wire["samples"][0]["time"]["unknown"] = true.into();
    assert!(serde_json::from_value::<StrokeInput>(wire).is_err());
    let mut wire = serde_json::to_value(&q).unwrap();
    wire["samples"][0]["tilt"] = serde_json::json!([0, 0]);
    assert!(serde_json::from_value::<StrokeInput>(wire).is_err());
    let asset = canvas(64, 32);
    let tiles = BTreeMap::new();
    q.samples[0].time = Time {
        numerator: 0,
        denominator: 2,
    };
    assert_eq!(
        paint(&asset, &tiles, &q, &Budget::default(), || false)
            .err()
            .unwrap()
            .code,
        "paint_sample"
    );
    q.samples[0].time = Time::new(0, 1).unwrap();
    q.finish = false;
    let mut partial = asset.clone();
    let mut partial_tiles = tiles.clone();
    perform(&mut partial, &mut partial_tiles, &q);
    let mut changed = q.clone();
    changed.brush.radius = 3.;
    assert_eq!(
        paint(
            &partial,
            &partial_tiles,
            &changed,
            &Budget::default(),
            || false
        )
        .err()
        .unwrap()
        .code,
        "paint_stroke"
    );
    changed = q.clone();
    changed.samples = vec![];
    assert_eq!(
        paint(
            &partial,
            &partial_tiles,
            &changed,
            &Budget::default(),
            || false
        )
        .err()
        .unwrap()
        .code,
        "paint_sample"
    );
    q.samples = vec![sample(30., 10., 0), sample(35., 10., 1)];
    assert_eq!(
        paint(
            &asset,
            &tiles,
            &q,
            &Budget {
                max_touched_tiles: 1,
                ..Budget::default()
            },
            || false
        )
        .err()
        .unwrap()
        .code,
        "budget"
    );
    q.samples = vec![sample(1., 1., 0); 257];
    assert_eq!(
        paint(&asset, &tiles, &q, &Budget::default(), || false)
            .err()
            .unwrap()
            .code,
        "budget"
    );
    let (doc, _) = paint_document();
    let mut snapshot = doc.snapshot().clone();
    let tile = Arc::new(Tile::transparent());
    snapshot.paint_tiles = (0..513)
        .map(|i| (format!("sha256:{i:064x}"), tile.clone()))
        .collect();
    assert_eq!(snapshot.validate().unwrap_err().code, "budget");
    let mut snapshot = doc.snapshot().clone();
    let asset = Arc::new(canvas(1, 1));
    snapshot.paint_assets = (0..65)
        .map(|i| (format!("sha256:{i:064x}"), asset.clone()))
        .collect();
    assert_eq!(snapshot.validate().unwrap_err().code, "budget");
    let mut asset = canvas(33, 1);
    let mut tile = Tile::full_mask();
    let Tile::Mask { coverage_le, .. } = &mut tile else {
        unreachable!()
    };
    coverage_le[1] = 0;
    let hash = tile.content_id().unwrap();
    let tiles = BTreeMap::from([(hash.clone(), Arc::new(tile))]);
    asset.layers[0]
        .mask
        .0
        .insert(Coordinate { x: 1, y: 0 }, hash);
    assert_eq!(asset.validate_tiles(&tiles).unwrap_err().code, "paint_tile");
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn decimal_segment_endpoints_do_not_receive_a_duplicate_opacity_dab() {
    let source = canvas(16, 2);
    let tiles = BTreeMap::new();
    for (a, b, spacing) in [(5.1, 0.1, 0.625), (10., 0.1, 1.2375)] {
        let mut q = input(
            1,
            paint_color([1., 0., 0., 0.5]),
            vec![sample(a, 0.5, 0), sample(b, 0.5, 1)],
        );
        q.brush.radius = 1.;
        q.brush.spacing = spacing;
        let complete = paint(&source, &tiles, &q, &Budget::default(), || false).unwrap();
        // Each independent segment is eight spacings long: initial + eight = nine.
        assert_eq!(complete.report.emitted_dabs, 9);
        q.finish = false;
        let checkpoint = paint(&source, &tiles, &q, &Budget::default(), || false).unwrap();
        assert_eq!(checkpoint.report.emitted_dabs, 9);
        q.samples.clear();
        q.finish = true;
        let finish = paint(
            &checkpoint.asset,
            &checkpoint.tiles,
            &q,
            &Budget::default(),
            || false,
        )
        .unwrap();
        assert_eq!(finish.report.emitted_dabs, 0);
        assert!(finish.tiles.is_empty());
        assert_eq!(complete.asset, finish.asset);
    }
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn bake_retry_identity_includes_canvas_and_source_even_for_identical_pixels() {
    use render_core::{document::Command, fixtures};
    let (mut doc, first) = paint_document();
    let p = fixtures::principal();
    let mut alternate = doc.snapshot().paint_assets[&first.source_asset]
        .as_ref()
        .clone();
    alternate.layers[0].name = "Different authored source".into();
    let second = alternate.content_id().unwrap();
    let add = fixtures::request(
        &doc,
        "paint-bake-other-canvas",
        vec![
            Command::PutPaintAsset { asset: alternate },
            Command::SetPaintCanvas {
                canvas: Id(901),
                source_asset: None,
                asset: Some(second.clone()),
            },
        ],
    )
    .unwrap();
    doc.execute(&p, &add).unwrap();
    let request = BakeRequest {
        version: 0,
        base_revision: doc.snapshot().revision().unwrap(),
        idempotency_key: "paint-bake-source-identity".into(),
        canvas: first.canvas,
        source_asset: first.source_asset,
        budget: BakeBudget::default(),
        max_added_bytes: 4 * 1024 * 1024,
    };
    let prepared = prepare_bake(&doc, &p, &request, || false).unwrap();
    doc.execute(&p, &prepared.transaction).unwrap();
    let before = canonical(&doc).unwrap();
    let mut other = request.clone();
    other.canvas = Id(901);
    other.source_asset = second;
    assert_eq!(
        prepare_bake(&doc, &p, &other, || false).err().unwrap().code,
        "idempotency_mismatch"
    );
    let mut missing = request.clone();
    missing.canvas = Id(999);
    assert_eq!(
        prepare_bake(&doc, &p, &missing, || false)
            .err()
            .unwrap()
            .code,
        "idempotency_mismatch"
    );
    assert_eq!(canonical(&doc).unwrap(), before);
    assert_eq!(
        canonical(
            &prepare_bake(&doc, &p, &request, || false)
                .unwrap()
                .transaction
        )
        .unwrap(),
        canonical(&prepared.transaction).unwrap()
    );
    other.idempotency_key = "paint-bake-other-accepted".into();
    other.base_revision = doc.snapshot().revision().unwrap();
    let other = prepare_bake(&doc, &p, &other, || false).unwrap();
    assert_eq!(other.report.image, prepared.report.image);
    assert_ne!(other.report.source, prepared.report.source);
    doc.execute(&p, &other.transaction).unwrap();
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn srgb8_image_transfer_preserves_native_oracle_and_bounds_analytic_error() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/tiled-painting/srgb8-native-baseline.json"
    ))
    .unwrap();
    let image = image::RgbaImage::from_fn(256, 1, |x, _| image::Rgba([x as u8; 4]));
    let color = render_core::textures::Pyramid::new(&image, TextureRole::SrgbColor);
    let data = render_core::textures::Pyramid::new(&image, TextureRole::LinearData);
    let mut previous = -1_f32;
    let mut error = 0_f64;
    for code in 0..256 {
        let actual = color.levels[0].rgba[code];
        let oracle = reference["bits"][code].as_u64().unwrap() as u32;
        assert_eq!(actual[0].to_bits(), oracle);
        assert_eq!(actual[0], actual[1]);
        assert_eq!(actual[1], actual[2]);
        assert!(actual[0] > previous);
        previous = actual[0];
        let linear = code as f32 / 255.;
        assert_eq!(actual[3], linear);
        assert_eq!(data.levels[0].rgba[code], [linear; 4]);
        let x = code as f64 / 255.;
        let analytic = if x <= 0.04045 {
            x / 12.92
        } else {
            libm::pow((x + 0.055) / 1.055, 2.4)
        };
        error = error.max((f64::from(actual[0]) - analytic).abs());
    }
    assert_eq!(color.levels[0].rgba[0], [0.; 4]);
    assert_eq!(color.levels[0].rgba[255], [1.; 4]);
    assert!(error < 3e-7, "8-bit transfer error {error}");
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn mip_selection_matches_native_footprints_and_analytic_constant_levels() {
    use render_core::textures::{Filter, Level, MinFilter, Pyramid, Sampler, Wrap};
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/tiled-painting/mip-native-baseline.json"
    ))
    .unwrap();
    let pairs: Vec<[u32; 2]> = serde_json::from_value(reference["pairs"].clone()).unwrap();
    let colors = [
        [1., 0., 0., 1.],
        [0., 1., 0., 1.],
        [0., 0., 1., 1.],
        [1.; 4],
    ];
    let image = Pyramid {
        levels: [8, 4, 2, 1]
            .into_iter()
            .zip(colors)
            .map(|(size, color)| Level {
                width: size,
                height: size,
                rgba: vec![color; (size * size) as usize],
            })
            .collect(),
    };
    let sampler = Sampler {
        wrap_s: Wrap::Clamp,
        wrap_t: Wrap::Clamp,
        mag: Filter::Linear,
        min: MinFilter::LinearMipLinear,
    };
    for [rho, lod] in pairs {
        let rho = f32::from_bits(rho);
        let lod = f32::from_bits(lod).clamp(0., 3.);
        let first = lod.floor() as usize;
        let next = (first + 1).min(3);
        let t = lod.fract();
        let expected = std::array::from_fn(|i| colors[first][i] * (1. - t) + colors[next][i] * t);
        let actual = image
            .sample(
                &sampler,
                glam::Vec2::splat(0.5),
                glam::Vec2::new(rho / 8., 0.),
                glam::Vec2::ZERO,
            )
            .to_array();
        assert_eq!(actual, expected, "native mip weights for footprint {rho}");
    }
    for (rho, expected) in [
        (0., colors[0]),
        (1., colors[0]),
        (2., colors[1]),
        (4., colors[2]),
        (8., colors[3]),
        (f32::INFINITY, colors[3]),
    ] {
        assert_eq!(
            image
                .sample(
                    &sampler,
                    glam::Vec2::splat(0.5),
                    glam::Vec2::new(rho / 8., 0.),
                    glam::Vec2::ZERO
                )
                .to_array(),
            expected
        );
    }
}
