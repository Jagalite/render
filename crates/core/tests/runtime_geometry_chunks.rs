use render_core::{
    fixtures,
    render::{
        self,
        chunks::{CHUNK_LEN, Chunks},
    },
};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn boundaries_and_bidirectional_iteration() {
    for len in [0, 1, 63, 64, 65, 127, 128, 129, 4097] {
        let values: Vec<_> = (0..len).collect();
        let chunks = Chunks::from(values.clone());
        assert_eq!(chunks.len(), len);
        assert_eq!(chunks.chunk_count(), len.div_ceil(CHUNK_LEN));
        assert_eq!(chunks.is_empty(), len == 0);
        assert_eq!(chunks.iter().copied().collect::<Vec<_>>(), values);
        assert_eq!(
            chunks.iter().rev().copied().collect::<Vec<_>>(),
            values.iter().rev().copied().collect::<Vec<_>>()
        );
        assert_eq!(chunks.get(len), None);
        assert_eq!(chunks.get(usize::MAX), None);
        let mut iter = chunks.iter();
        let (mut first, mut last) = (0, len);
        while first < last {
            assert_eq!(iter.len(), last - first);
            assert_eq!(iter.next(), Some(&first));
            first += 1;
            if first < last {
                last -= 1;
                assert_eq!(iter.next_back(), Some(&last));
            }
        }
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next_back(), None);
    }
}
#[derive(Debug)]
struct Counted {
    value: usize,
    clones: Arc<AtomicUsize>,
}
impl Clone for Counted {
    fn clone(&self) -> Self {
        self.clones.fetch_add(1, Ordering::Relaxed);
        Self {
            value: self.value,
            clones: self.clones.clone(),
        }
    }
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn only_touched_payloads_clone_and_old_versions_survive() {
    let count = Arc::new(AtomicUsize::new(0));
    let make = |value| Counted {
        value,
        clones: count.clone(),
    };
    let original = Chunks::from((0..4097).map(make).collect::<Vec<_>>());
    let copy = original.clone();
    assert!(original.shares_root(&copy));
    assert_eq!(count.load(Ordering::Relaxed), 0);
    let (patched, cost) = original
        .replaced(
            BTreeMap::from([(63, make(9000)), (64, make(9001)), (4096, make(9002))]),
            || false,
        )
        .unwrap();
    assert_eq!(cost.root_references, 65);
    assert_eq!(cost.chunks, 3);
    assert_eq!(cost.elements, 129);
    assert_eq!(count.load(Ordering::Relaxed), 129);
    assert_eq!(original.shared_chunks(&patched), 62);
    for i in 0..original.len() {
        assert_eq!(original[i].value, i);
        assert_eq!(copy[i].value, i);
    }
    assert_eq!(patched[63].value, 9000);
    assert_eq!(patched[64].value, 9001);
    assert_eq!(patched[4096].value, 9002);
    let (empty, cost) = original.replaced(BTreeMap::new(), || false).unwrap();
    assert!(empty.shares_root(&original));
    assert_eq!(cost, Default::default());
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn invalid_and_every_cancel_checkpoint_leave_sources_unchanged() {
    let source = Chunks::from((0..130).collect::<Vec<_>>());
    let changes = BTreeMap::from([(0, 500), (1, 501), (65, 565), (129, 629)]);
    let mut calls = 0;
    let (_, cost) = source
        .replaced(changes.clone(), || {
            calls += 1;
            false
        })
        .unwrap();
    assert_eq!(cost.elements, 130);
    assert_eq!(cost.chunks, 3);
    for stop in 1..=calls {
        let mut current = 0;
        assert_eq!(
            source
                .replaced(changes.clone(), || {
                    current += 1;
                    current == stop
                })
                .unwrap_err()
                .code,
            "cancelled"
        );
        assert_eq!(
            source.iter().copied().collect::<Vec<_>>(),
            (0..130).collect::<Vec<_>>()
        );
    }
    for bad in [130, usize::MAX] {
        assert_eq!(
            source
                .replaced(BTreeMap::from([(0, 999), (bad, 999)]), || false)
                .unwrap_err()
                .code,
            "index"
        );
    }
    assert_eq!(source[0], 0);
    assert_eq!(
        source.replaced(BTreeMap::new(), || true).unwrap_err().code,
        "cancelled"
    );
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn evaluated_geometry_clones_share_payloads_and_preserve_render() {
    let (scene, settings, expected) = fixtures::diffuse_plane().unwrap();
    let original = render::render(&scene, &settings, || false).unwrap();
    let mut copied = scene.clone();
    for instance in &mut copied.instances {
        let geometry = (*instance.geometry).clone();
        assert!(geometry.triangles.shares_root(&instance.geometry.triangles));
        assert!(geometry.bvh.nodes.shares_root(&instance.geometry.bvh.nodes));
        instance.geometry = Arc::new(geometry);
    }
    let image = render::render(&copied, &settings, || false).unwrap();
    assert_eq!(image.linear, original.linear);
    assert_eq!(image.depth, original.depth);
    assert_eq!(image.normals, original.normals);
    assert_eq!(image.objects, original.objects);
    for p in image.linear {
        for i in 0..3 {
            assert!((p[i] - expected[i]).abs() < 1e-5);
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn geometry_payload_copy_costs_include_nested_allocations() {
    use glam::{DVec3, Vec2};
    use render::{Bvh, Triangle};
    let triangles = Chunks::from(
        (0..4096)
            .map(|i| {
                let p = DVec3::new((i % 64) as f64, (i / 64) as f64, 0.);
                Triangle {
                    positions: [p, p + DVec3::X * 0.5, p + DVec3::Y * 0.5],
                    uv: [Vec2::ZERO; 3],
                    normals: None,
                    tangents: None,
                    uv_sets: vec![[Vec2::ZERO; 3]; 2],
                    colors: None,
                }
            })
            .collect::<Vec<_>>(),
    );
    let bvh = Bvh::build(&triangles.iter().map(Triangle::bounds).collect::<Vec<_>>());
    let mut update = triangles[63].clone();
    update.positions[0].z = 0.25;
    let (changed, tcost) = triangles
        .replaced(BTreeMap::from([(63, update)]), || false)
        .unwrap();
    assert_eq!(tcost.chunks, 1);
    assert_eq!(tcost.elements, 64);
    assert_eq!(tcost.root_references, 64);
    assert_eq!(triangles.shared_chunks(&changed), 63);
    assert_eq!(triangles[63].positions[0].z, 0.);
    let nested_triangle_bytes: usize = triangles
        .iter()
        .take(64)
        .map(|t| t.uv_sets.len() * size_of::<[Vec2; 3]>())
        .sum();
    assert_eq!(nested_triangle_bytes, 3072);
    for i in 0..64 {
        assert_ne!(triangles[i].uv_sets.as_ptr(), changed[i].uv_sets.as_ptr());
    }
    assert_eq!(triangles[64].uv_sets.as_ptr(), changed[64].uv_sets.as_ptr());
    // Storage-only no-op node replacement: no refit or deformation claim.
    let node = bvh.nodes.len() - 1;
    let (nodes, ncost) = bvh
        .nodes
        .replaced(BTreeMap::from([(node, bvh.nodes[node].clone())]), || false)
        .unwrap();
    assert_eq!(ncost.chunks, 1);
    assert_eq!(ncost.elements, bvh.nodes.len() % 64);
    let nested_node_bytes: usize = bvh
        .nodes
        .iter()
        .skip(node / 64 * 64)
        .map(|n| n.items.len() * size_of::<usize>())
        .sum();
    for i in node / 64 * 64..bvh.nodes.len() {
        assert_eq!(nodes[i].items, bvh.nodes[i].items);
        if !nodes[i].items.is_empty() {
            assert_ne!(nodes[i].items.as_ptr(), bvh.nodes[i].items.as_ptr());
        }
    }
    let resources = serde_json::json!({
        "triangles":triangles.len(),"nodes":bvh.nodes.len(),"chunk_len":CHUNK_LEN,
        "triangle_root_references_copied":tcost.root_references,"triangle_chunks_copied":tcost.chunks,"triangle_elements_cloned":tcost.elements,
        "triangle_nested_bytes_cloned":nested_triangle_bytes,"triangle_retained_layout_bytes":triangles.retained_layout_bytes(),
        "node_root_references_copied":ncost.root_references,"node_chunks_copied":ncost.chunks,"node_elements_cloned":ncost.elements,
        "node_nested_bytes_cloned":nested_node_bytes,"node_retained_layout_bytes":bvh.nodes.retained_layout_bytes(),
        "scope":"layout excludes Self, Arc headers, allocator overhead and nested allocations; nested copied payloads reported separately; node replacement is not refit"
    });
    #[cfg(not(target_arch = "wasm32"))]
    println!("CHUNK_RESOURCE {}", resources);
    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_test::console_log!("CHUNK_RESOURCE {}", resources);
}
