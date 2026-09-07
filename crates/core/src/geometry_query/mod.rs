//! Read-only asset-local queries. Document revision admission is global; only tree
//! traversal is local. A successful query publishes at most one disposable index.
mod types;
use crate::{
    Error, Result, canonical, digest,
    document::Snapshot,
    geometry::Mesh,
    render::{Bounds, Bvh},
};
use glam::DVec3;
use std::{io::Write, sync::Arc};
pub use types::*;

fn check(cancelled: &mut impl FnMut() -> bool) -> Result<()> {
    if cancelled() {
        Err(Error::new("cancelled", "spatial query cancelled"))
    } else {
        Ok(())
    }
}
fn budget(message: &str) -> Error {
    Error::new("budget", message)
}
struct Counter {
    bytes: u64,
    max: u64,
}
impl Write for Counter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() as u64 > self.max - self.bytes {
            return Err(std::io::Error::other("source byte cap"));
        }
        self.bytes += bytes.len() as u64;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
/// Preflight serialization without constructing an unbounded JSON Value. Sorting
/// object keys and promoting f32 values in canonical() can change byte length;
/// this preliminary bound limits input traversal, followed by an exact bound.
fn admit_source(mesh: &Mesh, hash: &str, limits: &Budget) -> Result<(u64, u64, u64)> {
    if mesh.positions.len() > 65_536
        || mesh.corners.len() > 393_216
        || mesh.edges.len() > 393_216
        || mesh.faces() > 131_072
        || mesh.point_ids.len() > 65_536
        || mesh.edge_ids.len() > 393_216
        || mesh.corner_ids.len() > 393_216
        || mesh.face_ids.len() > 131_072
    {
        return Err(budget("mesh exceeds spatial profile element counts"));
    }
    let mut count = Counter {
        bytes: 0,
        max: limits.max_source_bytes,
    };
    serde_json::to_writer(&mut count, mesh)
        .map_err(|_| budget("source exceeds canonical byte cap"))?;
    mesh.validate()?;
    let mut triangles = 0;
    let mut triangulation_work = 0;
    for offsets in mesh.face_offsets.windows(2) {
        let corners = offsets[1] - offsets[0];
        if corners > 256 {
            return Err(budget(
                "spatial profile allows at most 256 corners per face",
            ));
        }
        triangles += u64::from(corners - 2);
        triangulation_work += u64::from(corners).pow(3);
        if triangulation_work > limits.max_triangulation_work {
            return Err(budget("aggregate triangulation work charge exceeds budget"));
        }
    }
    if triangles > 131_072 {
        return Err(budget("derived triangle cap"));
    }
    for i in 0..mesh.positions.len() {
        if mesh.positions.get(i).abs().max_element() > 1e9 {
            return Err(Error::new(
                "query_metric",
                "source position exceeds local meter profile",
            ));
        }
    }
    let charge = tree_charge(mesh.positions.len() as u64)
        + tree_charge(triangles)
        + 64 * mesh.corners.len() as u64
        + 4096;
    if charge > limits.max_build_bytes {
        return Err(budget("spatial index build byte charge exceeds budget"));
    }
    let bytes = canonical(mesh)?;
    // Assert the bound on the actual canonical representation as well.
    if bytes.len() as u64 > limits.max_source_bytes {
        return Err(budget("canonical source byte cap"));
    }
    if digest(&bytes) != hash {
        return Err(Error::new(
            "integrity",
            "mesh key does not match source content",
        ));
    }
    Ok((bytes.len() as u64, charge, triangulation_work))
}
fn tree_charge(n: u64) -> u64 {
    // Leaves contain 2..=4 items except the one-item root, so nodes <= n.
    // The 512n node allowance covers the builder Vec, immutable chunk payloads
    // and root/Arc overhead coexisting during conversion (including tiny trees).
    // Item split capacities and sort scratch cost 8 bytes per slot per level.
    // Another 128n covers bounds, triangle payloads and builder temporaries.
    let levels = if n == 0 {
        0
    } else {
        64 - n.leading_zeros() as u64
    };
    n * (640 + 8 * (levels + 2))
}
#[derive(Debug)]
struct Triangle {
    corners: [u32; 3],
    face: usize,
}
#[derive(Debug)]
struct Index {
    source: Arc<Mesh>,
    hash: String,
    points: Bvh,
    surfaces: Bvh,
    triangles: Vec<Triangle>,
    source_bytes: u64,
    charge: u64,
    triangulation_work: u64,
}
impl Index {
    fn build(
        source: Arc<Mesh>,
        hash: String,
        source_bytes: u64,
        charge: u64,
        triangulation_work: u64,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<Self> {
        check(cancelled)?;
        let corners = source.triangles()?;
        check(cancelled)?;
        let mut faces = vec![0; source.corners.len()];
        for (face, range) in source.face_offsets.windows(2).enumerate() {
            faces[range[0] as usize..range[1] as usize].fill(face);
        }
        let triangles: Vec<_> = corners
            .into_iter()
            .map(|corners| Triangle {
                face: faces[corners[0] as usize],
                corners,
            })
            .collect();
        let mut bounds = Vec::with_capacity(source.positions.len());
        for i in 0..source.positions.len() {
            check(cancelled)?;
            let p = source.positions.get(i);
            bounds.push(Bounds { min: p, max: p });
        }
        let points = Bvh::build_compact(&bounds);
        drop(bounds);
        check(cancelled)?;
        let bounds: Vec<_> = triangles
            .iter()
            .map(|t| {
                let p = t.corners.map(|c| {
                    source
                        .positions
                        .get(source.corners[c as usize].vertex as usize)
                });
                check(cancelled)?;
                let area2 = (p[1] - p[0]).cross(p[2] - p[0]).length_squared();
                if area2 == 0.0 || !area2.is_finite() {
                    return Err(Error::new(
                        "degenerate",
                        "query triangle has no finite nonzero area",
                    ));
                }
                let ab = p[1] - p[0];
                let ac = p[2] - p[0];
                let scale = ab.abs().max_element().max(ac.abs().max_element());
                let normalized = (ab / scale).cross(ac / scale);
                if !normalized.is_finite() || normalized.abs().max_element() == 0.0 {
                    return Err(Error::new(
                        "degenerate",
                        "triangle cannot be resolved in normalized query frame",
                    ));
                }
                Ok(Bounds {
                    min: p[0].min(p[1]).min(p[2]),
                    max: p[0].max(p[1]).max(p[2]),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let surfaces = Bvh::build_compact(&bounds);
        check(cancelled)?;
        Ok(Self {
            source,
            hash,
            points,
            surfaces,
            triangles,
            source_bytes,
            charge,
            triangulation_work,
        })
    }
    fn retained_bytes(&self) -> u64 {
        let tree_bytes = |tree: &Bvh| {
            tree.nodes.retained_layout_bytes()
                + tree
                    .nodes
                    .iter()
                    .map(|n| n.items.capacity() * size_of::<usize>())
                    .sum::<usize>()
        };
        (size_of::<Self>()
            + self.hash.capacity()
            + self.triangles.capacity() * size_of::<Triangle>()
            + tree_bytes(&self.points)
            + tree_bytes(&self.surfaces)) as u64
    }
    fn query(
        &self,
        query: &Query,
        limits: &Budget,
        cost: &mut Cost,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<Hits> {
        let (q, radius) = query.metrics()?;
        let is_points = matches!(query, Query::PointsInSphere { .. });
        let tree = if is_points {
            &self.points
        } else {
            &self.surfaces
        };
        let mut best_distance = Distance::limit(radius);
        let mut points = Vec::new();
        let mut surface: Option<SurfaceHit> = None;
        let mut ties = 0;
        let mut stack = Vec::new();
        if !tree.nodes.is_empty() {
            stack.push(0);
        }
        while let Some(slot) = stack.pop() {
            check(cancelled)?;
            if cost.visited_nodes == limits.max_visited_nodes {
                return Err(budget("visited node cap"));
            }
            cost.visited_nodes += 1;
            let node = &tree.nodes[slot];
            if lower_distance(node.bounds, q) > best_distance {
                continue;
            }
            if let Some([a, b]) = node.children {
                let da = lower_distance(tree.nodes[a].bounds, q);
                let db = lower_distance(tree.nodes[b].bounds, q);
                if da <= db {
                    stack.extend([b, a]);
                } else {
                    stack.extend([a, b]);
                }
                continue;
            }
            for &item in &node.items {
                check(cancelled)?;
                if cost.item_tests == limits.max_item_tests {
                    return Err(budget("exact item test cap"));
                }
                cost.item_tests += 1;
                if is_points {
                    let p = self.source.positions.get(item);
                    let distance_metric = Distance::delta(q - p);
                    if distance_metric <= best_distance {
                        if points.len() as u64 == limits.max_hits {
                            return Err(budget("sphere hit cap"));
                        }
                        points.push(PointHit {
                            point: ElementId(self.source.point_ids[item]),
                            position_meters: p.to_array(),
                            distance_meters: distance_metric.meters(),
                        });
                    }
                } else {
                    let t = &self.triangles[item];
                    let p = t.corners.map(|c| {
                        self.source
                            .positions
                            .get(self.source.corners[c as usize].vertex as usize)
                    });
                    let (position, barycentric, normal, distance_metric) = closest(p, q);
                    if !position.is_finite()
                        || !normal.is_finite()
                        || barycentric.iter().any(|v| !v.is_finite())
                        || !distance_metric.meters().is_finite()
                    {
                        return Err(Error::new("numerical", "non-finite spatial surface result"));
                    }
                    if distance_metric > best_distance {
                        continue;
                    }
                    let hit = SurfaceHit {
                        face: ElementId(self.source.face_ids[t.face]),
                        corners: t
                            .corners
                            .map(|c| ElementId(self.source.corner_ids[c as usize])),
                        barycentric,
                        position_meters: position.to_array(),
                        geometric_normal: normal.to_array(),
                        distance_meters: distance_metric.meters(),
                    };
                    if distance_metric < best_distance || surface.is_none() {
                        ties = 1;
                        surface = Some(hit);
                    } else {
                        ties += 1;
                        if surface
                            .as_ref()
                            .is_some_and(|old| (hit.face, hit.corners) < (old.face, old.corners))
                        {
                            surface = Some(hit);
                        }
                    }
                    best_distance = distance_metric;
                }
            }
        }
        points.sort_by_key(|p| p.point);
        Ok(if is_points {
            Hits::PointsInSphere { points }
        } else {
            Hits::NearestSurface {
                surface,
                exact_tied_triangles: ties,
            }
        })
    }
}
/// Normal squared distances preserve the established arithmetic. Below the
/// normal f64 range, compare a scaled Euclidean length instead of allowing
/// underflow to collapse distinct points onto a zero-radius boundary.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
struct Distance {
    squared: bool,
    value: f64,
}
impl Distance {
    const INFINITY: Self = Self {
        squared: true,
        value: f64::INFINITY,
    };
    fn limit(radius: f64) -> Self {
        let square = radius * radius;
        if square >= f64::MIN_POSITIVE {
            Self {
                squared: true,
                value: square,
            }
        } else {
            Self {
                squared: false,
                value: radius.abs(),
            }
        }
    }
    fn delta(delta: DVec3) -> Self {
        let square = delta.length_squared();
        if square >= f64::MIN_POSITIVE {
            Self {
                squared: true,
                value: square,
            }
        } else {
            let scale = delta.abs().max_element();
            let value = if scale == 0.0 {
                0.0
            } else {
                scale * libm::sqrt((delta / scale).length_squared())
            };
            Self {
                squared: false,
                value,
            }
        }
    }
    fn meters(self) -> f64 {
        if self.squared {
            libm::sqrt(self.value)
        } else {
            self.value
        }
    }
}
fn lower_distance(bounds: Bounds, q: DVec3) -> Distance {
    Distance::delta(q - q.clamp(bounds.min, bounds.max))
}
fn closest(p: [DVec3; 3], q: DVec3) -> (DVec3, [f64; 3], DVec3, Distance) {
    let [a, b, c] = p;
    let ab = b - a;
    let ac = c - a;
    // Work in a dimensionless triangle frame. Squaring the original area
    // loses precision in the subnormal range even for an admitted valid face.
    let scale = ab.abs().max_element().max(ac.abs().max_element());
    let basis_b = ab / scale;
    let basis_c = ac / scale;
    let n = basis_b.cross(basis_c);
    let unit_scale = n / n.abs().max_element();
    let normal = unit_scale / libm::sqrt(unit_scale.length_squared());
    let area = n.dot(normal);
    let relative = (q - a) / scale;
    let projected = relative - normal * relative.dot(normal);
    let v = projected.cross(basis_c).dot(normal) / area;
    let w = basis_b.cross(projected).dot(normal) / area;
    let bary = [1.0 - v - w, v, w];
    let position = a + ab * v + ac * w;
    let mut best = (position, bary, Distance::INFINITY);
    if bary.iter().all(|v| *v >= 0.0) {
        best.2 = Distance::delta(q - position);
    }
    for (i, j) in [(0, 1), (1, 2), (2, 0)] {
        let edge = p[j] - p[i];
        let edge_scale = edge.abs().max_element();
        let direction = edge / edge_scale;
        let t =
            (((q - p[i]) / edge_scale).dot(direction) / direction.length_squared()).clamp(0.0, 1.0);
        let point = p[i] + edge * t;
        let d = Distance::delta(q - point);
        if d < best.2 {
            let mut bary = [0.; 3];
            bary[i] = 1. - t;
            bary[j] = t;
            best = (point, bary, d);
        }
    }
    (best.0, best.1, normal, best.2)
}
#[derive(Debug, Clone, Default)]
pub struct Cache {
    index: Option<Arc<Index>>,
}
impl Cache {
    pub fn query(
        &mut self,
        snapshot: &Snapshot,
        request: &Request,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<Response> {
        check(&mut cancelled)?;
        if request.version != 0 {
            return Err(Error::new("version", "spatial query version must be zero"));
        }
        request.budget.validate()?;
        request.query.metrics()?;
        if snapshot.revision()? != request.base_revision {
            return Err(Error::new(
                "stale_revision",
                "spatial query revision is stale",
            ));
        }
        check(&mut cancelled)?;
        let source = snapshot
            .meshes
            .get(&request.mesh)
            .ok_or_else(|| Error::new("reference", "query mesh missing"))?;
        let cached = self.index.as_ref().filter(|i| i.hash == request.mesh);
        let identical_source = cached.is_some_and(|i| Arc::ptr_eq(&i.source, source));
        let (source_bytes, charge, triangulation_work) = if identical_source {
            let index = cached.expect("identical source requires cached index");
            if index.source_bytes > request.budget.max_source_bytes
                || index.charge > request.budget.max_build_bytes
                || index.triangulation_work > request.budget.max_triangulation_work
            {
                return Err(budget(
                    "cached source or build charge exceeds request budget",
                ));
            }
            (index.source_bytes, index.charge, index.triangulation_work)
        } else {
            admit_source(source, &request.mesh, &request.budget)?
        };
        check(&mut cancelled)?;
        let reused = cached.is_some();
        let index = match cached {
            Some(index) => Arc::clone(index),
            None => Arc::new(Index::build(
                Arc::clone(source),
                request.mesh.clone(),
                source_bytes,
                charge,
                triangulation_work,
                &mut cancelled,
            )?),
        };
        let mut cost = Cost {
            cache_reused: reused,
            whole_snapshot_validated: true,
            additional_source_identity_validation: !identical_source,
            source_bytes,
            build_byte_charge: charge,
            triangulation_work_charge: triangulation_work,
            built_points: if reused {
                0
            } else {
                source.positions.len() as u64
            },
            built_triangles: if reused {
                0
            } else {
                index.triangles.len() as u64
            },
            retained_index_layout_bytes: index.retained_bytes(),
            ..Cost::default()
        };
        let hits = index.query(&request.query, &request.budget, &mut cost, &mut cancelled)?;
        let response = Response {
            profile: "mesh-local-spatial-v1".into(),
            revision: request.base_revision.clone(),
            mesh: request.mesh.clone(),
            hits,
            cost,
        };
        let mut count = Counter {
            bytes: 0,
            max: request.budget.max_result_bytes,
        };
        serde_json::to_writer(&mut count, &response)
            .map_err(|_| budget("spatial result byte cap"))?;
        check(&mut cancelled)?;
        self.index = Some(index);
        Ok(response)
    }
}
