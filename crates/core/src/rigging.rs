//! Stable skeletons, affine linear blend skinning and deterministic constraints.
use crate::{
    Error, Id, Result,
    animation::{Pose, Target},
    document::*,
    geometry::*,
};
use glam::{DAffine3, DQuat, DVec3};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Joint {
    pub id: Id,
    pub parent: Option<Id>,
    pub rest: Transform,
    pub inverse_bind: Transform,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Goal {
    Point { position: [f64; 3] },
    Entity { entity: Id },
    Joint { joint: Id },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Constraint {
    CopyPosition {
        id: Id,
        joint: Id,
        target: Goal,
    },
    TwoBoneIk {
        id: Id,
        root: Id,
        mid: Id,
        tip: Id,
        target: Goal,
        pole: [f64; 3],
        tolerance: f64,
    },
}
impl Constraint {
    fn id(&self) -> Id {
        match self {
            Self::CopyPosition { id, .. } | Self::TwoBoneIk { id, .. } => *id,
        }
    }
    fn goal(&self) -> &Goal {
        match self {
            Self::CopyPosition { target, .. } | Self::TwoBoneIk { target, .. } => target,
        }
    }
    fn writes(&self) -> Vec<Id> {
        match self {
            Self::CopyPosition { joint, .. } => vec![*joint],
            Self::TwoBoneIk { root, mid, .. } => vec![*root, *mid],
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rig {
    pub joints: Vec<Joint>,
    pub constraints: Vec<Constraint>,
}
#[derive(Clone, Debug)]
pub struct Plan {
    pub ids: Vec<Id>,
    parents: Vec<Option<usize>>,
    rest: Vec<DAffine3>,
    pub inverse_bind: Vec<DAffine3>,
    constraints: Vec<Constraint>,
}
impl Rig {
    pub fn compile(&self) -> Result<Plan> {
        if self.joints.is_empty() || self.joints.len() > 1024 || self.constraints.len() > 256 {
            return Err(Error::new(
                "budget",
                "rig supports 1..1024 joints and at most 256 constraints",
            ));
        }
        let mut by_id = BTreeMap::new();
        for joint in &self.joints {
            if by_id.insert(joint.id, joint).is_some() {
                return Err(Error::new("duplicate_id", "duplicate joint"));
            }
            joint.rest.affine()?;
            joint.inverse_bind.inverse()?;
        }
        let mut ids = Vec::new();
        let mut index = BTreeMap::new();
        let mut parents = Vec::new();
        let mut rest = Vec::new();
        let mut inverse_bind = Vec::new();
        let mut globals: Vec<DAffine3> = Vec::new();
        while ids.len() < by_id.len() {
            let mut progress = false;
            for (&id, j) in &by_id {
                if index.contains_key(&id) {
                    continue;
                }
                if j.parent.is_some_and(|p| !by_id.contains_key(&p)) {
                    return Err(Error::new("reference", "joint parent missing"));
                }
                if j.parent.is_some_and(|p| !index.contains_key(&p)) {
                    continue;
                }
                let parent = j.parent.map(|p| index[&p]);
                let local = j.rest.affine()?;
                let world = parent.map_or(local, |i: usize| globals[i] * local);
                let bind = j.inverse_bind.affine()?;
                let error = (world * bind)
                    .to_cols_array()
                    .iter()
                    .zip(DAffine3::IDENTITY.to_cols_array())
                    .map(|(a, b)| (a - b).abs())
                    .fold(0., f64::max);
                if error > 1e-8 {
                    return Err(Error::new(
                        "bind_pose",
                        "inverse bind must invert the global rest transform",
                    )
                    .with_context("joint", format!("{:032x}", id.0)));
                }
                index.insert(id, ids.len());
                ids.push(id);
                parents.push(parent);
                rest.push(local);
                inverse_bind.push(bind);
                globals.push(world);
                progress = true;
            }
            if !progress {
                return Err(Error::new("rig_cycle", "joint parent cycle"));
            }
        }
        let mut constraint_ids = BTreeSet::new();
        let mut writer = BTreeMap::new();
        for (i, c) in self.constraints.iter().enumerate() {
            if !constraint_ids.insert(c.id()) {
                return Err(Error::new("duplicate_id", "duplicate constraint"));
            }
            for joint in c.writes() {
                if !index.contains_key(&joint) {
                    return Err(Error::new("reference", "constraint joint missing"));
                }
                if writer.insert(joint, i).is_some() {
                    return Err(Error::new(
                        "constraint_conflict",
                        "multiple constraints write one joint",
                    ));
                }
            }
            if let Constraint::TwoBoneIk {
                root,
                mid,
                tip,
                pole,
                tolerance,
                ..
            } = c
            {
                if !index.contains_key(tip)
                    || by_id[mid].parent != Some(*root)
                    || by_id[tip].parent != Some(*mid)
                {
                    return Err(Error::new(
                        "ik_chain",
                        "IK requires a direct root-mid-tip chain",
                    ));
                }
                if pole.iter().any(|x| !x.is_finite())
                    || !tolerance.is_finite()
                    || !(1e-9..=0.1).contains(tolerance)
                {
                    return Err(Error::new("ik_policy", "invalid pole or tolerance"));
                }
            }
        }
        // Explicit constraint order is semantic. Reads cannot depend on a later
        // writer, including ancestor transforms; feedback islands are rejected.
        for (i, c) in self.constraints.iter().enumerate() {
            let mut reads = vec![];
            if let Goal::Joint { joint } = c.goal() {
                if !index.contains_key(joint) {
                    return Err(Error::new("reference", "constraint target joint missing"));
                }
                reads.push(*joint);
            }
            for write in c.writes() {
                if let Some(p) = by_id[&write].parent
                    && !c.writes().contains(&p)
                {
                    reads.push(p);
                }
            }
            for mut read in reads {
                loop {
                    if writer.get(&read).is_some_and(|j| *j >= i) {
                        return Err(Error::new(
                            "constraint_cycle",
                            "constraint reads itself or a later writer",
                        )
                        .with_context("constraint", format!("{:032x}", c.id().0)));
                    }
                    let Some(p) = by_id[&read].parent else { break };
                    read = p;
                }
            }
            if let Goal::Point { position } = c.goal()
                && position.iter().any(|x| !x.is_finite())
            {
                return Err(Error::new("constraint", "nonfinite rig-space goal"));
            }
        }
        Ok(Plan {
            ids,
            parents,
            rest,
            inverse_bind,
            constraints: self.constraints.clone(),
        })
    }
}
impl Plan {
    fn globals(&self, locals: &[DAffine3]) -> Vec<DAffine3> {
        let mut globals = Vec::with_capacity(locals.len());
        for (i, local) in locals.iter().enumerate() {
            globals.push(self.parents[i].map_or(*local, |p| globals[p] * *local));
        }
        globals
    }
    pub fn evaluate(
        &self,
        rig: Id,
        poses: &BTreeMap<Target, Pose>,
        mut entity_goal: impl FnMut(Id) -> Result<DVec3>,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<Vec<DAffine3>> {
        let mut locals = Vec::new();
        for (i, &joint) in self.ids.iter().enumerate() {
            if cancelled() {
                return Err(Error::new("cancelled", "joint evaluation cancelled"));
            }
            locals.push(
                self.rest[i]
                    * poses
                        .get(&Target::Joint { rig, joint })
                        .map(Pose::affine)
                        .transpose()?
                        .unwrap_or(DAffine3::IDENTITY),
            );
        }
        let index: BTreeMap<_, _> = self
            .ids
            .iter()
            .enumerate()
            .map(|(i, id)| (*id, i))
            .collect();
        let mut world = self.globals(&locals);
        for c in &self.constraints {
            if cancelled() {
                return Err(Error::new("cancelled", "constraint evaluation cancelled"));
            }
            let goal = match c.goal() {
                Goal::Point { position } => DVec3::from_array(*position),
                Goal::Joint { joint } => world[index[joint]].translation,
                Goal::Entity { entity } => entity_goal(*entity)?,
            };
            let result = (|| -> Result<()> {
                match c {
                    Constraint::CopyPosition { joint, .. } => {
                        let j = index[joint];
                        let mut desired = world[j];
                        desired.translation = goal;
                        locals[j] =
                            self.parents[j].map_or(desired, |p| world[p].inverse() * desired);
                        world = self.globals(&locals);
                    }
                    Constraint::TwoBoneIk {
                        root,
                        mid,
                        tip,
                        pole,
                        tolerance,
                        ..
                    } => {
                        let (r, m, t) = (index[root], index[mid], index[tip]);
                        let a = world[r].translation;
                        let b = world[m].translation;
                        let z = world[t].translation;
                        let l1 = a.distance(b);
                        let l2 = b.distance(z);
                        let distance = a.distance(goal);
                        if l1 < 1e-9
                            || l2 < 1e-9
                            || distance < 1e-9
                            || distance > l1 + l2 + *tolerance
                            || distance < (l1 - l2).abs() - *tolerance
                        {
                            return Err(Error::new(
                                "ik_unreachable",
                                "goal outside the two-bone reachable annulus",
                            ));
                        }
                        let direction = (goal - a) / distance;
                        let mut bend = DVec3::from_array(*pole) - a;
                        bend -= direction * bend.dot(direction);
                        if bend.length_squared() < 1e-18 {
                            return Err(Error::new(
                                "ik_pole",
                                "pole is collinear with the goal axis",
                            ));
                        }
                        bend = bend.normalize();
                        let along = ((l1 * l1 - l2 * l2 + distance * distance) / (2. * distance))
                            .clamp(-l1, l1);
                        let middle =
                            a + direction * along + bend * (l1 * l1 - along * along).max(0.).sqrt();
                        let rotation =
                            DQuat::from_rotation_arc((b - a).normalize(), (middle - a).normalize());
                        let desired = DAffine3::from_mat3_translation(
                            glam::DMat3::from_quat(rotation) * world[r].matrix3,
                            a,
                        );
                        locals[r] =
                            self.parents[r].map_or(desired, |p| world[p].inverse() * desired);
                        world = self.globals(&locals);
                        let b = world[m].translation;
                        let z = world[t].translation;
                        let rotation =
                            DQuat::from_rotation_arc((z - b).normalize(), (goal - b).normalize());
                        let desired = DAffine3::from_mat3_translation(
                            glam::DMat3::from_quat(rotation) * world[m].matrix3,
                            b,
                        );
                        locals[m] = world[r].inverse() * desired;
                        world = self.globals(&locals);
                        if world[t].translation.distance(goal) > *tolerance {
                            return Err(Error::new(
                                "ik_residual",
                                "IK residual exceeds the authored tolerance",
                            ));
                        }
                    }
                }
                Ok(())
            })();
            result.map_err(|e| e.with_context("constraint", format!("{:032x}", c.id().0)))?;
        }
        if world
            .iter()
            .any(|t| !t.is_finite() || t.matrix3.determinant().abs() < 1e-15)
        {
            return Err(Error::new(
                "singular_pose",
                "evaluated joint transform is singular/nonfinite",
            ));
        }
        Ok(world)
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Influence {
    pub joint: Id,
    pub weight: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Skin {
    pub rig: Id,
    pub topology: String,
    pub mesh_bind: Transform,
    #[serde(with = "crate::geometry::local_map")]
    pub weights: BTreeMap<u64, Vec<Influence>>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Morph {
    pub id: Id,
    pub topology: String,
    #[serde(with = "crate::geometry::local_map")]
    pub offsets: BTreeMap<u64, [f64; 3]>,
}
impl Skin {
    pub fn validate(&self, mesh: &Mesh, rig: &Rig) -> Result<()> {
        if mesh.face_offsets.windows(2).any(|w| w[1] - w[0] != 3) {
            return Err(Error::new(
                "deformation_profile",
                "skin binding requires explicit triangle realization before binding",
            ));
        }
        if crate::groom::topology(mesh)? != self.topology {
            return Err(Error::new("stale_topology", "skin topology changed"));
        }
        self.mesh_bind.inverse()?;
        if self.weights.len() != mesh.positions.len() {
            return Err(Error::new(
                "skin_weights",
                "every mesh point requires explicit weights",
            ));
        }
        let joints: BTreeSet<_> = rig.joints.iter().map(|j| j.id).collect();
        for id in &mesh.point_ids {
            let weights = self
                .weights
                .get(id)
                .ok_or_else(|| Error::new("skin_weights", "point weights missing"))?;
            if weights.is_empty() || weights.len() > 8 {
                return Err(Error::new("skin_weights", "point requires 1..8 influences"));
            }
            let mut ids = BTreeSet::new();
            let mut sum = 0.;
            for w in weights {
                if !joints.contains(&w.joint)
                    || !ids.insert(w.joint)
                    || !w.weight.is_finite()
                    || w.weight < 0.
                    || w.weight > 1.
                {
                    return Err(Error::new("skin_weights", "invalid influence"));
                }
                sum += w.weight;
            }
            if (sum - 1.).abs() > 1e-8 {
                return Err(Error::new(
                    "skin_weights",
                    "weights must sum to one; normalization is explicit",
                ));
            }
        }
        Ok(())
    }
}
impl Morph {
    pub fn validate(&self, mesh: &Mesh) -> Result<()> {
        if mesh.face_offsets.windows(2).any(|w| w[1] - w[0] != 3) {
            return Err(Error::new(
                "deformation_profile",
                "morph binding requires explicit triangle realization before binding",
            ));
        }
        if crate::groom::topology(mesh)? != self.topology {
            return Err(Error::new("stale_topology", "morph topology changed"));
        }
        let points: BTreeSet<_> = mesh.point_ids.iter().copied().collect();
        if self.offsets.is_empty()
            || self.offsets.iter().any(|(id, p)| {
                !points.contains(id) || p.iter().any(|x| !x.is_finite() || x.abs() > 1e6)
            })
        {
            return Err(Error::new("morph", "invalid sparse point offsets"));
        }
        Ok(())
    }
}
