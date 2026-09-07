//! Bounded derived refit state. Authored snapshots never contain mutable caches.
use super::{
    Asset, Budget, ChunkMap,
    validation::{check_cancel, triangle_ok},
};
use crate::{
    Error, Result,
    geometry::Mesh,
    render::{Bounds, Bvh, Geometry, Node, Triangle, chunks::Chunks},
};
use glam::DVec3;
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Clone, Debug, Default, Serialize)]
pub struct EvaluationReport {
    pub source_asset: String,
    pub base_mesh: String,
    pub basis_reused: bool,
    pub exact_geometry_reused: bool,
    pub global_snapshot_admission: bool,
    pub basis_byte_charge: u64,
    pub built_points: usize,
    pub built_triangles: usize,
    pub compared_block_references: usize,
    pub compared_sparse_entries: usize,
    pub changed_points: usize,
    pub changed_triangles: usize,
    pub refit_leaves: usize,
    pub refit_nodes: usize,
    pub leaf_triangle_bounds: usize,
    pub triangle_root_references_copied: usize,
    pub triangle_chunks_copied: usize,
    pub triangle_elements_cloned: usize,
    pub node_root_references_copied: usize,
    pub node_chunks_copied: usize,
    pub node_elements_cloned: usize,
    pub copy_byte_charge: u64,
    pub current_geometry_layout_bytes: usize,
    pub current_nested_layout_bytes: usize,
    pub basis_geometry_layout_bytes: usize,
    pub basis_nested_layout_bytes: usize,
    pub basis_mapping_layout_bytes: usize,
    pub basis_point_map_entries: usize,
    pub retained_chunk_references: usize,
    pub retained_chunk_references_copied: usize,
    pub instance_bound_vertices_scanned: usize,
}
#[derive(Debug)]
struct Basis {
    source: Arc<Mesh>,
    key: String,
    by_id: BTreeMap<u64, usize>,
    incident: Vec<Vec<usize>>,
    points: Vec<[usize; 3]>,
    parents: Vec<Option<usize>>,
    leaves: Vec<usize>,
    geometry: Arc<Geometry>,
    charge: u64,
    mapping_bytes: usize,
    nested_bytes: usize,
}
#[derive(Clone, Debug)]
struct State {
    basis: Arc<Basis>,
    asset: Arc<Asset>,
    geometry: Arc<Geometry>,
    hash: String,
    chunks: Arc<ChunkMap>,
    nested_bytes: usize,
}
#[derive(Clone, Default, Debug)]
pub struct Cache {
    current: Option<State>,
}
fn limit(message: &str) -> Error {
    Error::new("budget", message)
}
fn zero(v: DVec3) -> DVec3 {
    DVec3::from_array(v.to_array().map(|v| if v == 0. { 0. } else { v }))
}
fn empty() -> Bounds {
    Bounds {
        min: DVec3::splat(f64::INFINITY),
        max: DVec3::splat(f64::NEG_INFINITY),
    }
}
fn union(a: Bounds, b: Bounds) -> Bounds {
    Bounds {
        min: zero(a.min.min(b.min)),
        max: zero(a.max.max(b.max)),
    }
}
fn leaf_bounds(node: &Node, triangles: &Chunks<Triangle>) -> Bounds {
    node.items
        .iter()
        .fold(empty(), |b, &i| union(b, triangles[i].bounds()))
}
fn charge(points: usize, triangles: usize) -> u64 {
    let levels = usize::BITS - triangles.leading_zeros();
    4096 + points as u64 * 192 + triangles as u64 * (1536 + 8 * u64::from(levels))
}
fn nested_layout(g: &Geometry) -> usize {
    g.triangles
        .iter()
        .map(|t| t.uv_sets.capacity() * size_of::<[glam::Vec2; 3]>())
        .sum::<usize>()
        + g.bvh
            .nodes
            .iter()
            .map(|n| n.items.capacity() * size_of::<usize>())
            .sum::<usize>()
}
fn geometry_layout(g: &Geometry) -> usize {
    size_of::<Geometry>()
        + g.triangles.retained_layout_bytes()
        + g.bvh.nodes.retained_layout_bytes()
        + g.uv_attributes.capacity() * size_of::<crate::Id>()
}
impl Basis {
    fn build(
        source: Arc<Mesh>,
        key: String,
        budget: &Budget,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<Self> {
        check_cancel(cancelled)?;
        let charge = charge(source.positions.len(), source.face_ids.len());
        if charge > budget.max_basis_bytes {
            return Err(limit("sculpt basis construction charge exceeds budget"));
        }
        let mut geometry =
            Geometry::from_mesh_positions(&source, |i| zero(source.positions.get(i)))?;
        check_cancel(cancelled)?;
        // A sculpt basis fixes both partition and bottom-up reduction order.
        // Normalize bound zeros so undo/fresh/refit retain identical node values.
        let mut nodes = geometry.bvh.nodes.iter().cloned().collect::<Vec<_>>();
        let mut parents = vec![None; nodes.len()];
        let mut leaves = vec![usize::MAX; geometry.triangles.len()];
        for i in (0..nodes.len()).rev() {
            check_cancel(cancelled)?;
            if let Some([a, b]) = nodes[i].children {
                parents[a] = Some(i);
                parents[b] = Some(i);
                nodes[i].bounds = union(nodes[a].bounds, nodes[b].bounds);
            } else {
                nodes[i].bounds = leaf_bounds(&nodes[i], &geometry.triangles);
                for &item in &nodes[i].items {
                    leaves[item] = i;
                }
            }
        }
        geometry.bvh.nodes = nodes.into();
        let mut incident = vec![Vec::new(); source.positions.len()];
        let mut points = Vec::with_capacity(source.face_ids.len());
        for (i, face) in source.face_offsets.windows(2).enumerate() {
            check_cancel(cancelled)?;
            let start = face[0] as usize;
            let p = std::array::from_fn(|j| source.corners[start + j].vertex as usize);
            for &point in &p {
                incident[point].push(i);
            }
            points.push(p);
        }
        let by_id = source
            .point_ids
            .iter()
            .copied()
            .enumerate()
            .map(|(slot, id)| (id, slot))
            .collect();
        let nested_bytes = nested_layout(&geometry);
        let mut basis = Self {
            source,
            key,
            by_id,
            incident,
            points,
            parents,
            leaves,
            geometry: Arc::new(geometry),
            charge,
            mapping_bytes: 0,
            nested_bytes,
        };
        basis.mapping_bytes = basis.mapping_layout();
        Ok(basis)
    }
    fn mapping_layout(&self) -> usize {
        // The BTreeMap node allocator layout is not exposed; report it separately
        // as an entry count in future profiles rather than inventing bytes.
        self.incident.capacity() * size_of::<Vec<usize>>()
            + self
                .incident
                .iter()
                .map(|v| v.capacity() * size_of::<usize>())
                .sum::<usize>()
            + self.points.capacity() * size_of::<[usize; 3]>()
            + self.parents.capacity() * size_of::<Option<usize>>()
            + self.leaves.capacity() * size_of::<usize>()
    }
}
impl Cache {
    /// Snapshot admission is the caller's responsibility. Source and chunk hashes,
    /// all references and displaced faces must already have been validated.
    pub(crate) fn evaluate(
        &mut self,
        hash: &str,
        asset: Arc<Asset>,
        source: Arc<Mesh>,
        chunks: &ChunkMap,
        budget: &Budget,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<(Arc<Geometry>, EvaluationReport)> {
        budget.validate()?;
        check_cancel(cancelled)?;
        let reused = self
            .current
            .as_ref()
            .is_some_and(|s| s.basis.key == asset.base_mesh);
        let basis = if reused {
            self.current.as_ref().unwrap().basis.clone()
        } else {
            Arc::new(Basis::build(
                source,
                asset.base_mesh.clone(),
                budget,
                cancelled,
            )?)
        };
        if basis.charge > budget.max_basis_bytes {
            return Err(limit("sculpt warm basis charge exceeds budget"));
        }
        let old_chunks = if reused {
            Some(&self.current.as_ref().unwrap().chunks)
        } else {
            None
        };
        let old_asset = if reused {
            Some(&self.current.as_ref().unwrap().asset)
        } else {
            None
        };
        let previous = if reused {
            self.current.as_ref().unwrap().geometry.clone()
        } else {
            basis.geometry.clone()
        };
        let exact = reused && self.current.as_ref().unwrap().hash == hash;
        let mut report = EvaluationReport {
            source_asset: hash.into(),
            base_mesh: asset.base_mesh.clone(),
            basis_reused: reused,
            exact_geometry_reused: exact,
            global_snapshot_admission: true,
            basis_byte_charge: basis.charge,
            built_points: if reused {
                0
            } else {
                basis.source.positions.len()
            },
            built_triangles: if reused { 0 } else { basis.points.len() },
            basis_geometry_layout_bytes: geometry_layout(&basis.geometry),
            basis_nested_layout_bytes: basis.nested_bytes,
            basis_mapping_layout_bytes: basis.mapping_bytes,
            basis_point_map_entries: basis.by_id.len(),
            retained_chunk_references: asset.blocks.len(),
            instance_bound_vertices_scanned: basis.points.len() * 3,
            ..Default::default()
        };
        if exact {
            check_cancel(cancelled)?;
            report.current_geometry_layout_bytes = geometry_layout(&previous);
            report.current_nested_layout_bytes = self.current.as_ref().unwrap().nested_bytes;
            return Ok((previous, report));
        }
        let blocks = asset
            .blocks
            .keys()
            .chain(old_asset.into_iter().flat_map(|a| a.blocks.keys()))
            .copied()
            .collect::<BTreeSet<_>>();
        let mut changed_points = BTreeMap::new();
        for block in blocks {
            check_cancel(cancelled)?;
            report.compared_block_references += 1;
            let old_key = old_asset.and_then(|a| a.blocks.get(&block));
            let new_key = asset.blocks.get(&block);
            if old_key == new_key {
                continue;
            }
            let old = old_key
                .map(|k| {
                    old_chunks
                        .unwrap()
                        .get(k)
                        .ok_or_else(|| Error::new("reference", "previous sculpt chunk missing"))
                })
                .transpose()?;
            let new = new_key
                .map(|k| {
                    chunks
                        .get(k)
                        .ok_or_else(|| Error::new("reference", "new sculpt chunk missing"))
                })
                .transpose()?;
            let entries = old
                .into_iter()
                .chain(new)
                .flat_map(|c| c.points.iter().map(|p| p.slot))
                .collect::<BTreeSet<_>>();
            for slot in entries {
                check_cancel(cancelled)?;
                report.compared_sparse_entries += 1;
                let a = old.map_or([0.; 3], |c| c.delta(slot));
                let b = new.map_or([0.; 3], |c| c.delta(slot));
                if a == b {
                    continue;
                }
                let id = (block.0 << 6) | u64::from(slot);
                let point = *basis
                    .by_id
                    .get(&id)
                    .ok_or_else(|| Error::new("reference", "sculpt point absent from basis"))?;
                let position = asset.position(&basis.source, chunks, point)?;
                let before =
                    zero(basis.source.positions.get(point) + DVec3::from_array(a.map(f64::from)));
                if position != before {
                    changed_points.insert(point, position);
                }
            }
        }
        report.changed_points = changed_points.len();
        let changed_triangles = changed_points
            .keys()
            .flat_map(|&i| basis.incident[i].iter().copied())
            .collect::<BTreeSet<_>>();
        if changed_triangles.len() as u64 > budget.max_triangle_updates {
            return Err(limit("sculpt affected triangle budget"));
        }
        report.changed_triangles = changed_triangles.len();
        let mut nodes_to_refit = BTreeSet::new();
        let mut leaves = BTreeSet::new();
        for &triangle in &changed_triangles {
            check_cancel(cancelled)?;
            let leaf = basis.leaves[triangle];
            leaves.insert(leaf);
            let mut node = Some(leaf);
            while let Some(i) = node {
                if !nodes_to_refit.insert(i) {
                    break;
                }
                node = basis.parents[i];
            }
        }
        if nodes_to_refit.len() as u64 > budget.max_refit_nodes {
            return Err(limit("sculpt affected BVH node budget"));
        }
        report.refit_leaves = leaves.len();
        report.refit_nodes = nodes_to_refit.len();
        // Conservative portable charge includes touched payloads, nested UV/leaf
        // copies, candidate maps and all root-reference tables.
        let dirty_triangle_chunks = changed_triangles
            .iter()
            .map(|i| i / 64)
            .collect::<BTreeSet<_>>();
        let dirty_node_chunks = nodes_to_refit
            .iter()
            .map(|i| i / 64)
            .collect::<BTreeSet<_>>();
        let triangle_chunks = dirty_triangle_chunks.len();
        let node_chunks = dirty_node_chunks.len();
        let copy_charge = triangle_chunks as u64 * 64 * 768
            + node_chunks as u64 * 64 * 256
            + (previous.triangles.chunk_count() + previous.bvh.nodes.chunk_count()) as u64 * 32
            + (changed_triangles.len() + nodes_to_refit.len()) as u64 * 1024
            + asset.blocks.len() as u64 * 256;
        if copy_charge > budget.max_copy_bytes {
            return Err(limit(
                "sculpt payload/root/candidate copy charge exceeds budget",
            ));
        }
        report.copy_byte_charge = copy_charge;
        report.retained_chunk_references_copied = asset.blocks.len();
        let mut updates = BTreeMap::new();
        for triangle in changed_triangles {
            check_cancel(cancelled)?;
            let mut value = previous.triangles[triangle].clone();
            for j in 0..3 {
                if let Some(p) = changed_points.get(&basis.points[triangle][j]) {
                    value.positions[j] = *p;
                }
            }
            triangle_ok(value.positions)?;
            updates.insert(triangle, value);
        }
        let (triangles, tcost) = previous.triangles.replaced(updates, &mut *cancelled)?;
        let mut node_updates: BTreeMap<usize, Node> = BTreeMap::new();
        for i in nodes_to_refit.into_iter().rev() {
            check_cancel(cancelled)?;
            let mut node = previous.bvh.nodes[i].clone();
            node.bounds = if let Some([a, b]) = node.children {
                union(
                    node_updates
                        .get(&a)
                        .unwrap_or(&previous.bvh.nodes[a])
                        .bounds,
                    node_updates
                        .get(&b)
                        .unwrap_or(&previous.bvh.nodes[b])
                        .bounds,
                )
            } else {
                report.leaf_triangle_bounds += node.items.len();
                leaf_bounds(&node, &triangles)
            };
            node_updates.insert(i, node);
        }
        let (nodes, ncost) = previous.bvh.nodes.replaced(node_updates, &mut *cancelled)?;
        let geometry = Arc::new(Geometry {
            triangles,
            bvh: Bvh { nodes },
            uv_attributes: previous.uv_attributes.clone(),
            color_attribute: previous.color_attribute,
        });
        report.triangle_root_references_copied = tcost.root_references;
        report.triangle_chunks_copied = tcost.chunks;
        report.triangle_elements_cloned = tcost.elements;
        report.node_root_references_copied = ncost.root_references;
        report.node_chunks_copied = ncost.chunks;
        report.node_elements_cloned = ncost.elements;
        let mut nested_bytes = if reused {
            self.current.as_ref().unwrap().nested_bytes
        } else {
            basis.nested_bytes
        };
        for chunk in dirty_triangle_chunks {
            for i in chunk * 64..((chunk + 1) * 64).min(geometry.triangles.len()) {
                nested_bytes -=
                    previous.triangles[i].uv_sets.capacity() * size_of::<[glam::Vec2; 3]>();
                nested_bytes +=
                    geometry.triangles[i].uv_sets.capacity() * size_of::<[glam::Vec2; 3]>();
            }
        }
        for chunk in dirty_node_chunks {
            for i in chunk * 64..((chunk + 1) * 64).min(geometry.bvh.nodes.len()) {
                nested_bytes -= previous.bvh.nodes[i].items.capacity() * size_of::<usize>();
                nested_bytes += geometry.bvh.nodes[i].items.capacity() * size_of::<usize>();
            }
        }
        report.current_geometry_layout_bytes = geometry_layout(&geometry);
        report.current_nested_layout_bytes = nested_bytes;
        let retained_chunks = asset
            .blocks
            .values()
            .map(|key| {
                chunks
                    .get(key)
                    .cloned()
                    .map(|value| (key.clone(), value))
                    .ok_or_else(|| Error::new("reference", "sculpt retained chunk missing"))
            })
            .collect::<Result<ChunkMap>>()?;
        check_cancel(cancelled)?;
        self.current = Some(State {
            basis,
            asset,
            geometry: geometry.clone(),
            hash: hash.into(),
            chunks: Arc::new(retained_chunks),
            nested_bytes,
        });
        Ok((geometry, report))
    }
}
