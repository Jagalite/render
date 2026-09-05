use crate::{Error, Result, geometry::Mesh};
use glam::DVec3;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT_ARENA: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handle {
    arena: u64,
    slot: u32,
    generation: u32,
}
#[derive(Clone, Debug)]
struct Slot<T> {
    generation: u32,
    value: Option<T>,
}
#[derive(Clone, Debug)]
pub struct Arena<T> {
    slots: Vec<Slot<T>>,
    owner: u64,
    free: Vec<u32>,
}
impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T> Arena<T> {
    pub fn new() -> Self {
        Self {
            slots: vec![],
            owner: NEXT_ARENA.fetch_add(1, Ordering::Relaxed),
            free: vec![],
        }
    }
    pub fn insert(&mut self, value: T) -> Handle {
        if let Some(i) = self.free.pop() {
            let s = &mut self.slots[i as usize];
            s.value = Some(value);
            return Handle {
                arena: self.owner,
                slot: i,
                generation: s.generation,
            };
        }
        let h = Handle {
            arena: self.owner,
            slot: self.slots.len() as u32,
            generation: 0,
        };
        self.slots.push(Slot {
            generation: 0,
            value: Some(value),
        });
        h
    }
    pub fn get(&self, h: Handle) -> Result<&T> {
        if h.arena != self.owner {
            return Err(Error::new(
                "stale_handle",
                "handle belongs to another edit arena",
            ));
        }
        self.slots
            .get(h.slot as usize)
            .filter(|s| s.generation == h.generation)
            .and_then(|s| s.value.as_ref())
            .ok_or_else(|| Error::new("stale_handle", "arena handle is stale"))
    }
    pub fn get_mut(&mut self, h: Handle) -> Result<&mut T> {
        self.get(h)?;
        Ok(self.slots[h.slot as usize]
            .value
            .as_mut()
            .expect("validated slot"))
    }
    pub fn remove(&mut self, h: Handle) -> Result<T> {
        self.get(h)?;
        let s = &mut self.slots[h.slot as usize];
        s.generation = s.generation.saturating_add(1);
        if s.generation < u32::MAX {
            self.free.push(h.slot);
        }
        Ok(s.value.take().expect("validated live slot"))
    }
}
#[derive(Clone, Debug)]
struct Vertex {
    index: usize,
    edges: Vec<Handle>,
}
#[derive(Clone, Debug)]
struct Edge {
    vertices: [Handle; 2],
    corners: Vec<Handle>,
}
#[derive(Clone, Debug)]
struct Face {
    corners: Vec<Handle>,
}
#[derive(Clone, Debug)]
struct EditCorner {
    vertex: Handle,
    edge: Handle,
    face: Handle,
    next: Option<Handle>,
    radial_next: Option<Handle>,
}

/// Exclusive edit view. Radial edge fans retain any number of incident corners.
pub struct EditMesh {
    mesh: Mesh,
    vertices: Arena<Vertex>,
    edges: Arena<Edge>,
    faces: Arena<Face>,
    corners: Arena<EditCorner>,
    handles: Vec<Handle>,
    pub radial: Vec<Vec<usize>>,
    pub vertex_edges: Vec<Vec<usize>>,
    dirty_points: Vec<u64>,
}
pub struct EditReceipt {
    pub mesh: Mesh,
    pub preserved_points: Vec<u64>,
    pub changed_points: Vec<u64>,
}
impl EditMesh {
    pub fn new(mesh: &Mesh) -> Result<Self> {
        mesh.validate()?;
        let mut vertices = Arena::new();
        let handles: Vec<_> = (0..mesh.positions.len())
            .map(|i| {
                vertices.insert(Vertex {
                    index: i,
                    edges: vec![],
                })
            })
            .collect();
        let mut edges = Arena::new();
        let mut edge_handles = vec![];
        for edge in &mesh.edges {
            let pair = edge.map(|v| handles[v as usize]);
            let h = edges.insert(Edge {
                vertices: pair,
                corners: vec![],
            });
            for v in pair {
                vertices.get_mut(v)?.edges.push(h);
            }
            edge_handles.push(h);
        }
        let mut faces = Arena::new();
        let mut corners = Arena::new();
        for offsets in mesh.face_offsets.windows(2) {
            let face = faces.insert(Face { corners: vec![] });
            let mut ring = vec![];
            for c in &mesh.corners[offsets[0] as usize..offsets[1] as usize] {
                let edge = edge_handles[c.edge as usize];
                let h = corners.insert(EditCorner {
                    vertex: handles[c.vertex as usize],
                    edge,
                    face,
                    next: None,
                    radial_next: None,
                });
                edges.get_mut(edge)?.corners.push(h);
                ring.push(h);
            }
            for (i, &h) in ring.iter().enumerate() {
                corners.get_mut(h)?.next = Some(ring[(i + 1) % ring.len()]);
            }
            faces.get_mut(face)?.corners = ring;
        }
        for h in edge_handles {
            let fan = edges.get(h)?.corners.clone();
            for (i, &c) in fan.iter().enumerate() {
                corners.get_mut(c)?.radial_next = Some(fan[(i + 1) % fan.len()]);
            }
        }
        let mut radial = vec![vec![]; mesh.edges.len()];
        for (i, c) in mesh.corners.iter().enumerate() {
            radial[c.edge as usize].push(i);
        }
        let mut vertex_edges = vec![vec![]; mesh.positions.len()];
        for (i, e) in mesh.edges.iter().enumerate() {
            for &v in e {
                vertex_edges[v as usize].push(i);
            }
        }
        Ok(Self {
            mesh: mesh.clone(),
            vertices,
            edges,
            faces,
            corners,
            handles,
            radial,
            vertex_edges,
            dirty_points: vec![],
        })
    }
    pub fn vertex(&self, index: usize) -> Result<Handle> {
        self.handles
            .get(index)
            .copied()
            .ok_or_else(|| Error::new("index", "vertex out of range"))
    }
    pub fn move_vertex(&mut self, handle: Handle, position: DVec3) -> Result<()> {
        let index = self.vertices.get(handle)?.index;
        self.mesh.positions.set(index, position)?;
        self.dirty_points.push(self.mesh.point_ids[index]);
        Ok(())
    }
    pub fn commit(mut self) -> Result<EditReceipt> {
        self.validate_connectivity()?;
        self.mesh.validate()?;
        self.dirty_points.sort_unstable();
        self.dirty_points.dedup();
        Ok(EditReceipt {
            preserved_points: self.mesh.point_ids.clone(),
            changed_points: self.dirty_points,
            mesh: self.mesh,
        })
    }
    pub fn validate_connectivity(&self) -> Result<()> {
        for slot in &self.corners.slots {
            if let Some(c) = &slot.value {
                let v = self.vertices.get(c.vertex)?;
                let edge = self.edges.get(c.edge)?;
                let face = self.faces.get(c.face)?;
                let next = self.corners.get(
                    c.next
                        .ok_or_else(|| Error::new("topology", "open face cycle"))?,
                )?;
                let radial = self.corners.get(
                    c.radial_next
                        .ok_or_else(|| Error::new("topology", "open radial cycle"))?,
                )?;
                if v.index >= self.mesh.positions.len()
                    || !edge.vertices.contains(&c.vertex)
                    || next.face != c.face
                    || radial.edge != c.edge
                    || face.corners.len() < 3
                {
                    return Err(Error::new("topology", "invalid handle connectivity"));
                }
            }
        }
        Ok(())
    }
    pub fn manifold_candidate_accepts(&self) -> bool {
        self.radial.iter().all(|fan| fan.len() <= 2)
    }
    /// Heap capacities owned by connectivity only; excludes the canonical mesh,
    /// allocator metadata, and stack values. Intended for representation evidence.
    pub fn connectivity_capacity_bytes(&self) -> usize {
        fn vec_bytes<T>(v: &Vec<T>) -> usize {
            v.capacity() * size_of::<T>()
        }
        fn arena_bytes<T>(a: &Arena<T>) -> usize {
            vec_bytes(&a.slots) + vec_bytes(&a.free)
        }
        arena_bytes(&self.vertices)
            + arena_bytes(&self.edges)
            + arena_bytes(&self.faces)
            + arena_bytes(&self.corners)
            + vec_bytes(&self.handles)
            + vec_bytes(&self.dirty_points)
            + vec_bytes(&self.radial)
            + vec_bytes(&self.vertex_edges)
            + self.radial.iter().map(vec_bytes).sum::<usize>()
            + self.vertex_edges.iter().map(vec_bytes).sum::<usize>()
            + self
                .vertices
                .slots
                .iter()
                .filter_map(|s| s.value.as_ref())
                .map(|v| vec_bytes(&v.edges))
                .sum::<usize>()
            + self
                .edges
                .slots
                .iter()
                .filter_map(|s| s.value.as_ref())
                .map(|v| vec_bytes(&v.corners))
                .sum::<usize>()
            + self
                .faces
                .slots
                .iter()
                .filter_map(|s| s.value.as_ref())
                .map(|v| vec_bytes(&v.corners))
                .sum::<usize>()
    }
}
