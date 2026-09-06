//! Seam cuts and disk-chart validation. Runtime indices remain private.
use crate::{Error, Result, geometry::Mesh};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug)]
pub(super) struct Chart {
    pub faces: Vec<usize>,
    pub vertices: Vec<usize>, // representative corner per chart vertex
    pub corners: BTreeMap<usize, usize>,
    pub triangles: Vec<[usize; 3]>,
    pub boundary: Vec<usize>,
}
struct Union(Vec<usize>);
impl Union {
    fn new(n: usize) -> Self {
        Self((0..n).collect())
    }
    fn root(&mut self, mut i: usize) -> usize {
        while self.0[i] != i {
            self.0[i] = self.0[self.0[i]];
            i = self.0[i];
        }
        i
    }
    fn join(&mut self, a: usize, b: usize) {
        let a = self.root(a);
        let b = self.root(b);
        self.0[a.max(b)] = a.min(b);
    }
}
fn bad(message: &str) -> Error {
    Error::new("uv_topology", message)
}
fn check(cancel: &mut impl FnMut() -> bool) -> Result<()> {
    if cancel() {
        Err(Error::new("cancelled", "UV chart analysis cancelled"))
    } else {
        Ok(())
    }
}
pub(super) fn analyze(
    mesh: &Mesh,
    seams: &BTreeSet<u64>,
    mut cancel: impl FnMut() -> bool,
) -> Result<Vec<Chart>> {
    check(&mut cancel)?;
    mesh.validate()?;
    if mesh.positions.len() > 65536
        || mesh.edges.len() > 65536
        || mesh.corners.len() > 4096
        || mesh.faces() == 0
        || mesh.faces() > 1024
        || mesh.face_offsets.windows(2).any(|w| w[1] - w[0] > 256)
    {
        return Err(Error::new(
            "budget",
            "UV analysis exceeds mesh work profile",
        ));
    }
    let edge_indices: BTreeMap<_, _> = mesh
        .edge_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (*id, i))
        .collect();
    if seams.iter().any(|id| !edge_indices.contains_key(id)) {
        return Err(Error::new("stale_selection", "UV seam edge is absent"));
    }
    let mut face_of = vec![0; mesh.corners.len()];
    let mut next = vec![0; mesh.corners.len()];
    let mut fans = vec![Vec::new(); mesh.edges.len()];
    for (f, range) in mesh.face_offsets.windows(2).enumerate() {
        check(&mut cancel)?;
        let start = range[0] as usize;
        let end = range[1] as usize;
        for c in start..end {
            face_of[c] = f;
            next[c] = if c + 1 == end { start } else { c + 1 };
            fans[mesh.corners[c].edge as usize].push(c);
        }
    }
    let mut faces = Union::new(mesh.faces());
    let mut corners = Union::new(mesh.corners.len());
    for (e, fan) in fans.iter().enumerate() {
        check(&mut cancel)?;
        if fan.len() > 2 {
            return Err(bad("UV profile requires at most two surface uses per edge"));
        }
        if seams.contains(&mesh.edge_ids[e]) && fan.is_empty() {
            return Err(bad("a loose edge cannot be a UV seam"));
        }
        if let &[a, b] = fan.as_slice() {
            if face_of[a] == face_of[b]
                || mesh.corners[a].vertex != mesh.corners[next[b]].vertex
                || mesh.corners[next[a]].vertex != mesh.corners[b].vertex
            {
                return Err(bad("UV surface edge orientations are inconsistent"));
            }
            if !seams.contains(&mesh.edge_ids[e]) {
                faces.join(face_of[a], face_of[b]);
                corners.join(a, next[b]);
                corners.join(next[a], b);
            }
        }
    }
    let mut groups = BTreeMap::<usize, Vec<usize>>::new();
    for f in 0..mesh.faces() {
        groups.entry(faces.root(f)).or_default().push(f);
    }
    if groups.len() > 64 {
        return Err(Error::new("budget", "at most 64 UV charts"));
    }
    let triangles = mesh.triangles()?;
    let mut charts = Vec::new();
    for mut group in groups.into_values() {
        check(&mut cancel)?;
        group.sort_by_key(|&f| mesh.face_ids[f]);
        let face_set: BTreeSet<_> = group.iter().copied().collect();
        let members: Vec<_> = group
            .iter()
            .flat_map(|&f| mesh.face_offsets[f] as usize..mesh.face_offsets[f + 1] as usize)
            .collect();
        let mut roots = BTreeMap::<usize, Vec<usize>>::new();
        for c in members {
            roots.entry(corners.root(c)).or_default().push(c);
        }
        let mut sets: Vec<_> = roots.into_values().collect();
        for set in &mut sets {
            set.sort_by_key(|&c| mesh.corner_ids[c]);
        }
        sets.sort_by_key(|set| mesh.corner_ids[set[0]]);
        if sets.len() > 128 {
            return Err(Error::new("budget", "at most 128 vertices per UV chart"));
        }
        let vertices: Vec<_> = sets.iter().map(|set| set[0]).collect();
        let lookup: BTreeMap<_, _> = sets
            .iter()
            .enumerate()
            .flat_map(|(v, set)| set.iter().map(move |&c| (c, v)))
            .collect();
        let mut tris: Vec<_> = triangles
            .iter()
            .filter(|t| face_set.contains(&face_of[t[0] as usize]))
            .map(|t| t.map(|c| lookup[&(c as usize)]))
            .collect();
        tris.sort_by_key(|t| t.map(|v| mesh.corner_ids[vertices[v]]));
        if tris.len() > 256 {
            return Err(Error::new("budget", "at most 256 triangles per UV chart"));
        }
        let mut edges = BTreeMap::<(usize, usize), Vec<(usize, usize)>>::new();
        for t in &tris {
            if t[0] == t[1] || t[1] == t[2] || t[2] == t[0] {
                return Err(bad("collapsed UV chart triangle"));
            }
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                edges.entry((a.min(b), a.max(b))).or_default().push((a, b));
            }
        }
        if vertices.len() as i64 - edges.len() as i64 + tris.len() as i64 != 1 {
            return Err(bad(
                "UV chart must have disk Euler characteristic; add seams",
            ));
        }
        let mut outgoing = BTreeMap::new();
        let mut incoming = BTreeSet::new();
        for uses in edges.values() {
            match *uses.as_slice() {
                [(a, b)] => {
                    if outgoing.insert(a, b).is_some() || !incoming.insert(b) {
                        return Err(bad("UV boundary has a non-manifold fan"));
                    }
                }
                [(a, b), (c, d)] if a == d && b == c => (),
                _ => return Err(bad("UV chart edge fan is not an oriented manifold")),
            }
        }
        let start = *outgoing
            .keys()
            .next()
            .ok_or_else(|| bad("closed UV chart requires a seam"))?;
        let mut boundary = Vec::new();
        let mut seen = BTreeSet::new();
        let mut current = start;
        loop {
            check(&mut cancel)?;
            if !seen.insert(current) {
                if current == start {
                    break;
                }
                return Err(bad("UV boundary cycles incorrectly"));
            }
            boundary.push(current);
            current = *outgoing
                .get(&current)
                .ok_or_else(|| bad("UV boundary is open"))?;
        }
        if seen.len() != outgoing.len() || incoming != seen {
            return Err(bad("UV chart has multiple boundary loops"));
        }
        charts.push(Chart {
            faces: group,
            vertices,
            corners: lookup,
            triangles: tris,
            boundary,
        });
    }
    charts.sort_by_key(|c| mesh.face_ids[c.faces[0]]);
    check(&mut cancel)?;
    Ok(charts)
}
