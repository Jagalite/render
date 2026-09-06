//! Typed geometry and point-field graph. Public node/port kinds are independent
//! of execution maps; geometry modifiers reuse the direct modeling operations.
use crate::{Error, Id, Result, canonical, digest, geometry::*, modeling};
use glam::DVec3;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Type {
    Geometry,
    ScalarField,
    VectorField,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Node {
    InputGeometry,
    Mesh {
        asset: String,
    },
    Box {
        min: [f64; 3],
        max: [f64; 3],
    },
    Modify {
        geometry: Id,
        operation: modeling::Operation,
    },
    Group {
        geometry: Id,
        group: Id,
    },
    SetPositions {
        geometry: Id,
        positions: Id,
    },
    Position {
        domain: Domain,
    },
    Scalar {
        value: f64,
    },
    Vector {
        value: [f64; 3],
    },
    AddVector {
        a: Id,
        b: Id,
    },
    ScaleVector {
        vector: Id,
        scalar: Id,
    },
    Component {
        vector: Id,
        axis: u8,
    },
    Sine {
        scalar: Id,
    },
}
impl Node {
    fn inputs(&self) -> Vec<(Id, Type)> {
        match self {
            Self::Modify { geometry, .. } | Self::Group { geometry, .. } => {
                vec![(*geometry, Type::Geometry)]
            }
            Self::SetPositions {
                geometry,
                positions,
            } => vec![(*geometry, Type::Geometry), (*positions, Type::VectorField)],
            Self::AddVector { a, b } => vec![(*a, Type::VectorField), (*b, Type::VectorField)],
            Self::ScaleVector { vector, scalar } => {
                vec![(*vector, Type::VectorField), (*scalar, Type::ScalarField)]
            }
            Self::Component { vector, .. } => vec![(*vector, Type::VectorField)],
            Self::Sine { scalar } => vec![(*scalar, Type::ScalarField)],
            _ => vec![],
        }
    }
    fn output(&self) -> Type {
        match self {
            Self::Position { .. }
            | Self::Vector { .. }
            | Self::AddVector { .. }
            | Self::ScaleVector { .. } => Type::VectorField,
            Self::Scalar { .. } | Self::Component { .. } | Self::Sine { .. } => Type::ScalarField,
            _ => Type::Geometry,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub nodes: BTreeMap<Id, Node>,
    pub output: Id,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Graph {
    pub root: Group,
    pub groups: BTreeMap<Id, Group>,
    pub budget: modeling::Budget,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Receipt {
    pub graph_digest: String,
    pub output_digest: String,
    pub operations: Vec<modeling::Receipt>,
    pub executed_nodes: usize,
    pub output_bytes: usize,
    pub approximation: String,
}
#[derive(Clone, Debug)]
pub struct Evaluated {
    pub mesh: Mesh,
    pub receipt: Receipt,
}
impl Group {
    fn order(
        &self,
        assets: &BTreeMap<String, Arc<Mesh>>,
        groups: &BTreeMap<Id, Group>,
        allow_input: bool,
    ) -> Result<Vec<Id>> {
        if self.nodes.is_empty() || self.nodes.len() > 256 {
            return Err(Error::new("budget", "geometry group requires 1..256 nodes"));
        }
        for (id, node) in &self.nodes {
            for (input, expected) in node.inputs() {
                let found = self.nodes.get(&input).ok_or_else(|| {
                    Error::new("reference", "node input missing")
                        .with_context("node", format!("{:032x}", id.0))
                })?;
                if found.output() != expected {
                    return Err(Error::new("port_type", "node input/output type mismatch")
                        .with_context("node", format!("{:032x}", id.0)));
                }
            }
            match node {
                Node::InputGeometry if !allow_input => {
                    return Err(Error::new(
                        "port_type",
                        "root graph has no external geometry input",
                    ));
                }
                Node::Mesh { asset } if !assets.contains_key(asset) => {
                    return Err(Error::new("reference", "graph mesh asset missing"));
                }
                Node::Group { group, .. } if !groups.contains_key(group) => {
                    return Err(Error::new("reference", "node group missing"));
                }
                Node::Position { domain } if *domain != Domain::Point => {
                    return Err(Error::new(
                        "field_domain",
                        "position fields require the point domain",
                    ));
                }
                Node::Component { axis, .. } if *axis > 2 => {
                    return Err(Error::new("field", "component axis must be 0..2"));
                }
                Node::Scalar { value } if !value.is_finite() || value.abs() > 1e9 => {
                    return Err(Error::new("field", "invalid scalar constant"));
                }
                Node::Vector { value } if value.iter().any(|v| !v.is_finite() || v.abs() > 1e9) => {
                    return Err(Error::new("field", "invalid vector constant"));
                }
                _ => {}
            }
        }
        if self
            .nodes
            .get(&self.output)
            .is_none_or(|n| n.output() != Type::Geometry)
        {
            return Err(Error::new(
                "port_type",
                "group output must name a geometry node",
            ));
        }
        let mut order = vec![];
        let mut ready = BTreeSet::new();
        while ready.len() < self.nodes.len() {
            let before = ready.len();
            for (id, node) in &self.nodes {
                if !ready.contains(id)
                    && node.inputs().iter().all(|(input, _)| ready.contains(input))
                {
                    ready.insert(*id);
                    order.push(*id);
                }
            }
            if ready.len() == before {
                let first = *self
                    .nodes
                    .keys()
                    .find(|id| !ready.contains(id))
                    .expect("pending cycle");
                let mut path = vec![];
                let mut current = first;
                let mut seen = BTreeSet::new();
                while seen.insert(current) {
                    path.push(format!("{:032x}", current.0));
                    current = self.nodes[&current]
                        .inputs()
                        .iter()
                        .find(|(id, _)| !ready.contains(id))
                        .expect("cyclic predecessor")
                        .0;
                }
                path.push(format!("{:032x}", current.0));
                return Err(Error::new("graph_cycle", path.join(" -> ")));
            }
        }
        Ok(order)
    }
}
impl Graph {
    pub fn validate(&self, assets: &BTreeMap<String, Arc<Mesh>>) -> Result<()> {
        self.budget.validate()?;
        if self.groups.len() > 32 {
            return Err(Error::new("budget", "at most 32 node groups"));
        }
        self.root.order(assets, &self.groups, false)?;
        for group in self.groups.values() {
            group.order(assets, &self.groups, true)?;
        }
        fn visit(
            id: Id,
            groups: &BTreeMap<Id, Group>,
            path: &mut Vec<Id>,
            work: &mut usize,
        ) -> Result<()> {
            *work += 1;
            if *work > 4096 {
                return Err(Error::new(
                    "budget",
                    "group validation expansion exceeds 4096 visits",
                ));
            }
            if path.contains(&id) {
                return Err(Error::new(
                    "group_cycle",
                    format!("recursive group {:032x}", id.0),
                ));
            }
            if path.len() >= 8 {
                return Err(Error::new("budget", "group nesting exceeds eight"));
            }
            path.push(id);
            for node in groups[&id].nodes.values() {
                if let Node::Group { group, .. } = node {
                    visit(*group, groups, path, work)?;
                }
            }
            path.pop();
            Ok(())
        }
        let mut work = 0;
        for id in self.groups.keys() {
            visit(*id, &self.groups, &mut vec![], &mut work)?;
        }
        Ok(())
    }
    pub fn content_id(&self) -> Result<String> {
        Ok(digest(&canonical(self)?))
    }
    pub fn evaluate(
        &self,
        assets: &BTreeMap<String, Arc<Mesh>>,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<Evaluated> {
        if cancelled() {
            return Err(Error::new("cancelled", "procedural evaluation cancelled"));
        }
        self.validate(assets)?;
        let mut operations = vec![];
        let mut executed = 0;
        let mesh = self.run(
            &self.root,
            None,
            assets,
            &mut operations,
            &mut executed,
            &mut cancelled,
        )?;
        let receipt=Receipt{graph_digest:self.content_id()?,output_digest:mesh.content_id()?,operations,executed_nodes:executed,output_bytes:canonical(&mesh)?.len(),approximation:"typed point fields evaluated in topological order; explicit geometry group input/output; modifiers share direct operations; bounded eager geometry outputs; no simulation feedback".into()};
        Ok(Evaluated {
            mesh: (*mesh).clone(),
            receipt,
        })
    }
    fn run(
        &self,
        group: &Group,
        input: Option<Arc<Mesh>>,
        assets: &BTreeMap<String, Arc<Mesh>>,
        operations: &mut Vec<modeling::Receipt>,
        executed: &mut usize,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<Arc<Mesh>> {
        let order = group.order(assets, &self.groups, input.is_some())?;
        let mut meshes: BTreeMap<Id, Arc<Mesh>> = BTreeMap::new();
        for id in &order {
            if cancelled() {
                return Err(Error::new(
                    "cancelled",
                    "procedural node evaluation cancelled",
                ));
            }
            *executed += 1;
            if *executed > 4096 {
                return Err(Error::new(
                    "budget",
                    "expanded group execution exceeds 4096 nodes",
                ));
            }
            let output = match &group.nodes[id] {
                Node::InputGeometry => Some(input.as_ref().expect("validated group input").clone()),
                Node::Mesh { asset } => Some(assets[asset].clone()),
                Node::Box { min, max } => Some(Arc::new(modeling::box_mesh(*min, *max)?)),
                Node::Modify {
                    geometry,
                    operation,
                } => {
                    let (mesh, receipt) = modeling::apply(
                        &meshes[geometry],
                        operation,
                        &self.budget,
                        &mut *cancelled,
                    )?;
                    operations.push(receipt);
                    Some(Arc::new(mesh))
                }
                Node::Group { geometry, group } => Some(self.run(
                    &self.groups[group],
                    Some(meshes[geometry].clone()),
                    assets,
                    operations,
                    executed,
                    cancelled,
                )?),
                Node::SetPositions {
                    geometry,
                    positions,
                } => {
                    let mut mesh = (*meshes[geometry]).clone();
                    if mesh.positions.len() * order.len() > 16 * 1024 * 1024 {
                        return Err(Error::new("budget", "point-field work budget exceeded"));
                    }
                    let mut values = vec![];
                    for i in 0..mesh.positions.len() {
                        if cancelled() {
                            return Err(Error::new(
                                "cancelled",
                                "point field evaluation cancelled",
                            ));
                        }
                        let original = mesh.positions.get(i);
                        let mut scalar = BTreeMap::new();
                        let mut vector = BTreeMap::new();
                        for field in &order {
                            match &group.nodes[field] {
                                Node::Position { .. } => {
                                    vector.insert(*field, original);
                                }
                                Node::Scalar { value } => {
                                    scalar.insert(*field, *value);
                                }
                                Node::Vector { value } => {
                                    vector.insert(*field, DVec3::from_array(*value));
                                }
                                Node::AddVector { a, b } => {
                                    vector.insert(*field, vector[a] + vector[b]);
                                }
                                Node::ScaleVector {
                                    vector: v,
                                    scalar: s,
                                } => {
                                    vector.insert(*field, vector[v] * scalar[s]);
                                }
                                Node::Component { vector: v, axis } => {
                                    scalar.insert(*field, vector[v][*axis as usize]);
                                }
                                Node::Sine { scalar: s } => {
                                    scalar.insert(*field, scalar[s].sin());
                                }
                                _ => {}
                            }
                        }
                        let p = vector[positions];
                        if !p.is_finite() || p.abs().max_element() > 1e12 {
                            return Err(Error::new(
                                "field",
                                "position field produced invalid geometry",
                            )
                            .with_context("node", format!("{:032x}", id.0)));
                        }
                        values.push(p.to_array());
                    }
                    mesh.positions = Positions::F64(values);
                    mesh.attributes.retain(|_, a| {
                        !matches!(a.semantic.as_str(), "normal" | "tangent" | "tangent_sign")
                    });
                    mesh.validate()?;
                    mesh.triangles()?;
                    Some(Arc::new(mesh))
                }
                _ => None,
            };
            if let Some(mesh) = output {
                if mesh.positions.len() > self.budget.vertices as usize
                    || mesh.faces() > self.budget.faces as usize
                    || canonical(&mesh)?.len() as u64 > self.budget.bytes
                {
                    return Err(Error::new(
                        "budget",
                        "procedural geometry output budget exceeded",
                    ));
                }
                meshes.insert(*id, mesh);
                let retained: usize = meshes
                    .values()
                    .map(|m| m.positions.len() * 24 + m.corners.len() * 16)
                    .sum();
                if retained as u64 > self.budget.bytes {
                    return Err(Error::new(
                        "budget",
                        "procedural retained geometry estimate exceeds byte budget",
                    ));
                }
            }
        }
        meshes
            .remove(&group.output)
            .ok_or_else(|| Error::new("graph", "geometry output missing"))
    }
}
