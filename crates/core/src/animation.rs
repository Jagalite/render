//! Exact-time authored clips and deterministic pose deltas. Transform deltas are
//! T*R*S appended to the authored affine/rest transform, preserving its shear.
use crate::{Error, Id, Result, Time, canonical, digest, document::*};
use glam::{DAffine3, DQuat, DVec3};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}
pub(crate) fn ratio(n: i128, d: u128) -> Result<Time> {
    if d == 0 {
        return Err(Error::new("time", "zero time denominator"));
    }
    if n == 0 {
        return Time::new(0, 1);
    }
    let g = gcd(n.unsigned_abs(), d);
    Time::new(
        i64::try_from(n / i128::try_from(g).map_err(|_| Error::new("overflow", "time reduction"))?)
            .map_err(|_| Error::new("overflow", "time numerator"))?,
        u64::try_from(d / g).map_err(|_| Error::new("overflow", "time denominator"))?,
    )
}
impl Time {
    pub fn compare(self, other: Self) -> Result<std::cmp::Ordering> {
        self.seconds()?;
        other.seconds()?;
        Ok((i128::from(self.numerator) * i128::from(other.denominator))
            .cmp(&(i128::from(other.numerator) * i128::from(self.denominator))))
    }
    pub fn add_time(self, other: Self) -> Result<Self> {
        self.seconds()?;
        other.seconds()?;
        let g = gcd(self.denominator.into(), other.denominator.into());
        let a = u128::from(other.denominator) / g;
        let b = u128::from(self.denominator) / g;
        let n = (i128::from(self.numerator) * a as i128)
            .checked_add(i128::from(other.numerator) * b as i128)
            .ok_or_else(|| Error::new("overflow", "time addition"))?;
        ratio(n, u128::from(self.denominator) * a)
    }
    pub fn negate(self) -> Result<Self> {
        ratio(-i128::from(self.numerator), self.denominator.into())
    }
    pub fn subtract(self, other: Self) -> Result<Self> {
        // Do subtraction before narrowing so i64::MIN - i64::MIN is valid.
        self.seconds()?;
        other.seconds()?;
        let g = gcd(self.denominator.into(), other.denominator.into());
        let a = u128::from(other.denominator) / g;
        let b = u128::from(self.denominator) / g;
        let n = (i128::from(self.numerator) * a as i128)
            .checked_sub(i128::from(other.numerator) * b as i128)
            .ok_or_else(|| Error::new("overflow", "time subtraction"))?;
        ratio(n, u128::from(self.denominator) * a)
    }
    pub fn multiply(self, other: Self) -> Result<Self> {
        self.seconds()?;
        other.seconds()?;
        ratio(
            i128::from(self.numerator) * i128::from(other.numerator),
            u128::from(self.denominator) * u128::from(other.denominator),
        )
    }
    pub fn modulo(self, period: Self) -> Result<Self> {
        if period.numerator <= 0 {
            return Err(Error::new("time", "repeat period must be positive"));
        }
        self.seconds()?;
        period.seconds()?;
        let a = i128::from(self.numerator) * i128::from(period.denominator);
        let b = i128::from(period.numerator) * i128::from(self.denominator);
        ratio(
            a.rem_euclid(b),
            u128::from(self.denominator) * u128::from(period.denominator),
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Target {
    Entity { entity: Id },
    Joint { rig: Id, joint: Id },
    Morph { entity: Id, target: Id },
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Property {
    Translation,
    Scale,
    Euler { order: RotationOrder },
    Quaternion,
    MorphWeight,
    AbsoluteTranslation,
    AbsoluteScale,
    AbsoluteQuaternion,
}
impl Property {
    pub fn is_absolute(self) -> bool {
        matches!(
            self,
            Self::AbsoluteTranslation | Self::AbsoluteScale | Self::AbsoluteQuaternion
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Value {
    Scalar(f64),
    Vector([f64; 3]),
    Quaternion([f64; 4]),
}
impl Value {
    fn array(self) -> [f64; 4] {
        match self {
            Self::Scalar(v) => [v, 0., 0., 0.],
            Self::Vector(v) => [v[0], v[1], v[2], 0.],
            Self::Quaternion(v) => v,
        }
    }
    fn with_components(self, v: [f64; 4]) -> Self {
        match self {
            Self::Scalar(_) => Self::Scalar(v[0]),
            Self::Vector(_) => Self::Vector([v[0], v[1], v[2]]),
            Self::Quaternion(_) => Self::Quaternion(v),
        }
    }
    fn kind(self) -> u8 {
        match self {
            Self::Scalar(_) => 0,
            Self::Vector(_) => 1,
            Self::Quaternion(_) => 2,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Interpolation {
    Step,
    Linear,
    Cubic,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Key {
    pub time: Time,
    pub value: Value,
    pub incoming: Option<Value>,
    pub outgoing: Option<Value>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Track {
    pub id: Id,
    pub target: Target,
    pub property: Property,
    pub interpolation: Interpolation,
    pub keys: Vec<Key>,
}
impl Track {
    pub fn validate(&self) -> Result<()> {
        if self.keys.is_empty() || self.keys.len() > 16384 {
            return Err(Error::new(
                "budget",
                "animation channel needs 1..16384 keys",
            ));
        }
        let kind = match self.property {
            Property::Quaternion | Property::AbsoluteQuaternion => 2,
            Property::MorphWeight => 0,
            _ => 1,
        };
        if matches!(self.target, Target::Morph { .. })
            != matches!(self.property, Property::MorphWeight)
        {
            return Err(Error::new("channel", "morph target/property mismatch"));
        }
        for (i, k) in self.keys.iter().enumerate() {
            if Time::new(k.time.numerator, k.time.denominator)? != k.time {
                return Err(Error::new("time", "key times must be canonical rationals"));
            }
            if i > 0 && self.keys[i - 1].time.compare(k.time)? != std::cmp::Ordering::Less {
                return Err(Error::new("channel", "key times must strictly increase"));
            }
            if k.value.kind() != kind
                || k.value
                    .array()
                    .iter()
                    .any(|v| !v.is_finite() || v.abs() > 1e12)
            {
                return Err(Error::new("channel", "key value type or range invalid"));
            }
            if kind == 2 && (DQuat::from_array(k.value.array()).length_squared() - 1.).abs() > 1e-8
            {
                return Err(Error::new(
                    "quaternion",
                    "rotation keys must have unit length",
                ));
            }
            if matches!(self.interpolation, Interpolation::Cubic) {
                for tangent in [k.incoming, k.outgoing] {
                    let tangent = tangent.ok_or_else(|| {
                        Error::new(
                            "channel",
                            "cubic keys require incoming/outgoing derivatives per second",
                        )
                    })?;
                    if tangent.kind() != kind
                        || tangent
                            .array()
                            .iter()
                            .any(|v| !v.is_finite() || v.abs() > 1e12)
                    {
                        return Err(Error::new("channel", "invalid cubic tangent"));
                    }
                }
            } else if k.incoming.is_some() || k.outgoing.is_some() {
                return Err(Error::new(
                    "channel",
                    "non-cubic keys cannot carry ignored tangents",
                ));
            }
        }
        Ok(())
    }
    pub fn sample(&self, time: Time) -> Result<Value> {
        self.validate()?;
        self.sample_validated(time)
    }
    fn sample_validated(&self, time: Time) -> Result<Value> {
        time.seconds()?;
        let mut lo = 0;
        let mut hi = self.keys.len();
        while lo < hi {
            let mid = (lo + hi) / 2;
            if self.keys[mid].time.compare(time)? == std::cmp::Ordering::Greater {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        if lo == 0 {
            return Ok(self.keys[0].value);
        }
        if lo == self.keys.len() {
            return Ok(self.keys[lo - 1].value);
        }
        let a = &self.keys[lo - 1];
        let b = &self.keys[lo];
        let duration = b.time.subtract(a.time)?.seconds()?;
        let u = time.subtract(a.time)?.seconds()? / duration;
        if matches!(self.interpolation, Interpolation::Step) {
            return Ok(a.value);
        }
        if matches!(self.interpolation, Interpolation::Linear) && a.value.kind() == 2 {
            return Ok(Value::Quaternion(
                DQuat::from_array(a.value.array())
                    .slerp(DQuat::from_array(b.value.array()), u)
                    .normalize()
                    .to_array(),
            ));
        }
        let av = a.value.array();
        let bv = b.value.array();
        let v = match self.interpolation {
            Interpolation::Linear => std::array::from_fn(|i| av[i] * (1. - u) + bv[i] * u),
            Interpolation::Cubic => {
                let outgoing = a.outgoing.expect("validated tangent").array();
                let incoming = b.incoming.expect("validated tangent").array();
                std::array::from_fn(|i| {
                    (2. * u * u * u - 3. * u * u + 1.) * av[i]
                        + (u * u * u - 2. * u * u + u) * duration * outgoing[i]
                        + (-2. * u * u * u + 3. * u * u) * bv[i]
                        + (u * u * u - u * u) * duration * incoming[i]
                })
            }
            Interpolation::Step => unreachable!(),
        };
        if v.iter().any(|x| !x.is_finite()) {
            return Err(Error::new("numerics", "animation interpolation overflow"));
        }
        if a.value.kind() == 2 {
            let q = DQuat::from_array(v);
            if q.length_squared() < 1e-20 {
                return Err(Error::new(
                    "quaternion",
                    "cubic interpolation produced a zero rotation",
                ));
            }
            Ok(Value::Quaternion(q.normalize().to_array()))
        } else {
            Ok(a.value.with_components(v))
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Extrapolation {
    Clamp,
    Repeat,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeMap {
    pub rate: Time,
    pub offset: Time,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Clip {
    pub id: Id,
    pub start: Time,
    pub end: Time,
    pub extrapolation: Extrapolation,
    pub remap: TimeMap,
    pub tracks: Vec<Track>,
}
#[derive(Clone, Debug, Default)]
pub struct Pose {
    pub translation: Option<[f64; 3]>,
    pub scale: Option<[f64; 3]>,
    pub rotation: Option<TransformOp>,
    pub absolute: bool,
}
impl Pose {
    pub fn affine(&self) -> Result<DAffine3> {
        let mut t = Transform::default();
        if let Some(v) = self.translation {
            t.operations.push(TransformOp::TranslationMeters(v));
        }
        if let Some(r) = &self.rotation {
            t.operations.push(r.clone());
        }
        if let Some(v) = self.scale {
            t.operations.push(TransformOp::Scale(v));
        }
        t.affine()
    }
    pub(crate) fn apply(&self, rest: DAffine3) -> Result<DAffine3> {
        if self.absolute
            && (self.translation.is_none() || self.scale.is_none() || self.rotation.is_none())
        {
            return Err(Error::new("channel", "absolute pose requires complete TRS"));
        }
        Ok(if self.absolute {
            self.affine()?
        } else {
            rest * self.affine()?
        })
    }
    fn set(&mut self, property: Property, value: Value) -> Result<()> {
        self.absolute = property.is_absolute();
        match (property, value) {
            (Property::Translation | Property::AbsoluteTranslation, Value::Vector(v)) => {
                self.translation = Some(v)
            }
            (Property::Scale | Property::AbsoluteScale, Value::Vector(v)) => self.scale = Some(v),
            (Property::Euler { order }, Value::Vector(angles)) => {
                self.rotation = Some(TransformOp::RotationRadians { angles, order })
            }
            (Property::Quaternion | Property::AbsoluteQuaternion, Value::Quaternion(q)) => {
                self.rotation = Some(TransformOp::Quaternion(q))
            }
            _ => return Err(Error::new("channel", "pose value/property mismatch")),
        };
        Ok(())
    }
}
#[derive(Clone, Debug, Default)]
pub struct SampledClip {
    pub poses: BTreeMap<Target, Pose>,
    pub morphs: BTreeMap<(Id, Id), f64>,
}
impl Clip {
    pub fn validate(&self) -> Result<()> {
        if self.start.compare(self.end)? != std::cmp::Ordering::Less {
            return Err(Error::new("time", "clip duration must be positive"));
        }
        self.remap.rate.seconds()?;
        self.remap.offset.seconds()?;
        if self.tracks.is_empty() || self.tracks.len() > 4096 {
            return Err(Error::new("budget", "clip requires 1..4096 tracks"));
        }
        let mut ids = BTreeSet::new();
        let mut targets = BTreeSet::new();
        let mut spaces = BTreeMap::<Target, (bool, usize)>::new();
        let mut keys = 0;
        for t in &self.tracks {
            t.validate()?;
            keys += t.keys.len();
            if keys > 131072 {
                return Err(Error::new("budget", "clip key budget exceeded"));
            }
            let p = match t.property {
                Property::Translation | Property::AbsoluteTranslation => 0,
                Property::Scale | Property::AbsoluteScale => 1,
                Property::Euler { .. } | Property::Quaternion | Property::AbsoluteQuaternion => 2,
                Property::MorphWeight => 3,
            };
            let space = spaces
                .entry(t.target)
                .or_insert((t.property.is_absolute(), 0));
            if space.0 != t.property.is_absolute() {
                return Err(Error::new(
                    "channel",
                    "cannot mix absolute and delta channels on one target",
                ));
            }
            space.1 += 1;
            if !ids.insert(t.id) || !targets.insert((t.target, p)) {
                return Err(Error::new(
                    "channel",
                    "duplicate channel identity or conflicting target/property",
                ));
            }
            if t.keys[0].time.compare(self.start)? == std::cmp::Ordering::Less
                || t.keys.last().expect("keys").time.compare(self.end)?
                    == std::cmp::Ordering::Greater
            {
                return Err(Error::new("time", "channel keys outside clip bounds"));
            }
        }
        if spaces
            .values()
            .any(|(absolute, count)| *absolute && *count != 3)
        {
            return Err(Error::new(
                "channel",
                "absolute transforms require translation, quaternion and scale tracks",
            ));
        }
        Ok(())
    }
    pub fn sample(&self, time: Time, mut cancelled: impl FnMut() -> bool) -> Result<SampledClip> {
        if cancelled() {
            return Err(Error::new("cancelled", "clip sampling cancelled"));
        }
        self.validate()?;
        let mut at = time
            .multiply(self.remap.rate)?
            .add_time(self.remap.offset)?;
        at = match self.extrapolation {
            Extrapolation::Repeat => self.start.add_time(
                at.subtract(self.start)?
                    .modulo(self.end.subtract(self.start)?)?,
            )?,
            Extrapolation::Clamp => {
                if at.compare(self.start)? == std::cmp::Ordering::Less {
                    self.start
                } else if at.compare(self.end)? == std::cmp::Ordering::Greater {
                    self.end
                } else {
                    at
                }
            }
        };
        let mut sample = SampledClip::default();
        for track in &self.tracks {
            if cancelled() {
                return Err(Error::new("cancelled", "clip channel evaluation cancelled"));
            }
            let value = track.sample_validated(at)?;
            if let (Target::Morph { entity, target }, Value::Scalar(weight)) = (track.target, value)
            {
                if !(-8.0..=8.).contains(&weight) {
                    return Err(Error::new("morph", "morph weight outside [-8,8]"));
                }
                sample.morphs.insert((entity, target), weight);
            } else {
                sample
                    .poses
                    .entry(track.target)
                    .or_default()
                    .set(track.property, value)?;
            }
        }
        Ok(sample)
    }
    pub fn sample_digest(&self, time: Time) -> Result<String> {
        Ok(digest(&canonical(&(self, time))?))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub shading_frames: BTreeMap<Id, crate::rigging::FrameBinding>,
    pub clips: BTreeMap<Id, Clip>,
    pub rigs: BTreeMap<Id, crate::rigging::Rig>,
    pub skins: BTreeMap<Id, crate::rigging::Skin>,
    pub morphs: BTreeMap<Id, Vec<crate::rigging::Morph>>,
}
fn entity_mesh(s: &Snapshot, id: Id) -> Result<&crate::geometry::Mesh> {
    let e = s
        .entities
        .get(id)
        .ok_or_else(|| Error::new("reference", "deformation entity missing"))?;
    e.mesh
        .as_ref()
        .and_then(|key| s.meshes.get(key))
        .map(|m| m.as_ref())
        .ok_or_else(|| {
            Error::new(
                "deformation",
                "skin/morph requires an authored polygon mesh",
            )
        })
}
impl State {
    pub fn needs_v13(&self) -> bool {
        !self.shading_frames.is_empty()
            || self
                .morphs
                .values()
                .flatten()
                .any(|m| m.normal_offsets.is_some() || m.tangent_offsets.is_some())
    }
    pub fn needs_v11(&self) -> bool {
        self.clips
            .values()
            .any(|c| c.tracks.iter().any(|t| t.property.is_absolute()))
            || self.skins.values().any(|s| !s.inverse_binds.is_empty())
            || self
                .morphs
                .values()
                .flatten()
                .any(|m| m.default_weight != 0.)
    }
    pub fn merge(&mut self, other: &Self) -> Result<()> {
        if other
            .shading_frames
            .keys()
            .any(|k| self.shading_frames.contains_key(k))
            || other.clips.keys().any(|k| self.clips.contains_key(k))
            || other.rigs.keys().any(|k| self.rigs.contains_key(k))
            || other.skins.keys().any(|k| self.skins.contains_key(k))
            || other.morphs.keys().any(|k| self.morphs.contains_key(k))
        {
            return Err(Error::new(
                "duplicate_id",
                "animation merge would replace authored components",
            ));
        }
        self.shading_frames.extend(other.shading_frames.clone());
        self.clips.extend(other.clips.clone());
        self.rigs.extend(other.rigs.clone());
        self.skins.extend(other.skins.clone());
        self.morphs.extend(other.morphs.clone());
        Ok(())
    }
    pub fn validate(&self, s: &Snapshot) -> Result<()> {
        if self.shading_frames.len() > 256
            || self.clips.len() > 128
            || self.rigs.len() > 64
            || self.skins.len() > 256
            || self.morphs.len() > 256
        {
            return Err(Error::new("budget", "animation component budget exceeded"));
        }
        for (id, rig) in &self.rigs {
            if s.entities.get(*id).is_none() {
                return Err(Error::new("reference", "rig entity missing"));
            }
            rig.compile()?;
            for c in &rig.constraints {
                let target = match c {
                    crate::rigging::Constraint::CopyPosition { target, .. }
                    | crate::rigging::Constraint::TwoBoneIk { target, .. } => target,
                };
                if let crate::rigging::Goal::Entity { entity } = target
                    && s.entities.get(*entity).is_none()
                {
                    return Err(Error::new("reference", "constraint target entity missing"));
                }
            }
        }
        for (id, skin) in &self.skins {
            let mesh = entity_mesh(s, *id)?;
            let rig = self
                .rigs
                .get(&skin.rig)
                .ok_or_else(|| Error::new("reference", "skin rig missing"))?;
            skin.validate(mesh, rig)?;
        }
        for (id, morphs) in &self.morphs {
            let mesh = entity_mesh(s, *id)?;
            if morphs.len() > 256 {
                return Err(Error::new("budget", "at most 256 morph targets per mesh"));
            }
            let mut ids = BTreeSet::new();
            for m in morphs {
                if !ids.insert(m.id) {
                    return Err(Error::new("duplicate_id", "duplicate morph target"));
                }
                m.validate(mesh)?;
            }
        }
        for (id, frame) in &self.shading_frames {
            if !self.skins.contains_key(id) && !self.morphs.contains_key(id) {
                return Err(Error::new(
                    "deformation_frame",
                    "frame binding requires a deformation component",
                ));
            }
            frame.validate(entity_mesh(s, *id)?)?;
        }
        for (id, morphs) in &self.morphs {
            for m in morphs {
                for (offsets, normal) in [(&m.normal_offsets, true), (&m.tangent_offsets, false)] {
                    if let Some(offsets) = offsets {
                        let frame = self.shading_frames.get(id).ok_or_else(|| {
                            Error::new("reference", "morph directions require a frame binding")
                        })?;
                        if Some(offsets.attribute)
                            != if normal {
                                Some(frame.normal)
                            } else {
                                frame.tangent
                            }
                        {
                            return Err(Error::new(
                                "reference",
                                "morph direction attribute differs from frame binding",
                            ));
                        }
                    }
                }
            }
        }
        for (id, clip) in &self.clips {
            if *id != clip.id {
                return Err(Error::new("identity", "clip identity mismatch"));
            }
            clip.validate()?;
            for t in &clip.tracks {
                match t.target {
                    Target::Entity { entity } => {
                        if s.entities.get(entity).is_none() {
                            return Err(Error::new("reference", "animated entity missing"));
                        }
                    }
                    Target::Joint { rig, joint } => {
                        if !self
                            .rigs
                            .get(&rig)
                            .is_some_and(|r| r.joints.iter().any(|j| j.id == joint))
                        {
                            return Err(Error::new("reference", "animated joint missing"));
                        }
                    }
                    Target::Morph { entity, target } => {
                        if !self
                            .morphs
                            .get(&entity)
                            .is_some_and(|m| m.iter().any(|m| m.id == target))
                        {
                            return Err(Error::new("reference", "animated morph target missing"));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Receipt {
    pub authored_revision: String,
    pub clip: Id,
    pub time: Time,
    pub evaluation_digest: String,
    pub deformed_points: usize,
    pub approximation: String,
}
#[derive(Clone, Debug)]
pub struct Evaluated {
    pub snapshot: Snapshot,
    pub receipt: Receipt,
}
pub fn evaluate(
    snapshot: &Snapshot,
    clip: Id,
    time: Time,
    mut cancelled: impl FnMut() -> bool,
) -> Result<Evaluated> {
    if cancelled() {
        return Err(Error::new("cancelled", "animation evaluation cancelled"));
    }
    snapshot.validate()?;
    let state = snapshot
        .animation
        .as_ref()
        .ok_or_else(|| Error::new("animation", "document has no animation"))?;
    let selected = state
        .clips
        .get(&clip)
        .ok_or_else(|| Error::new("reference", "clip missing"))?;
    let sampled = selected.sample(time, &mut cancelled)?;
    let authored_revision = snapshot.revision()?;
    let evaluation_digest = digest(&canonical(&(
        &authored_revision,
        clip,
        time,
        if state.shading_frames.is_empty() {
            "affine-lbs-v0"
        } else {
            "affine-lbs-authored-frames-v1"
        },
    ))?);
    let (out, deformed_points) = evaluate_sample(snapshot, &sampled, &mut cancelled)?;
    Ok(Evaluated { snapshot: out, receipt: Receipt { authored_revision, clip, time, evaluation_digest, deformed_points,
        approximation: if state.shading_frames.is_empty() { "typed absolute TRS or affine rest plus pose deltas; morphs before LBS; explicit skin binds; flat geometric deformation normals" } else { "affine-lbs-authored-frames-v1: additive directions before skinning; inverse-transpose blended normals; linear blended tangents; determinant handedness; normalized directions; geometric fallback for unbound entities" }.into() } })
}
pub(crate) fn evaluate_rest(
    snapshot: &Snapshot,
    cancelled: impl FnMut() -> bool,
) -> Result<Snapshot> {
    evaluate_sample(snapshot, &SampledClip::default(), cancelled).map(|(s, _)| s)
}
fn evaluate_sample(
    snapshot: &Snapshot,
    sampled: &SampledClip,
    mut cancelled: impl FnMut() -> bool,
) -> Result<(Snapshot, usize)> {
    let state = snapshot
        .animation
        .as_ref()
        .ok_or_else(|| Error::new("animation", "missing state"))?;
    let (mut out, hidden) = snapshot.composition()?;
    for (target, pose) in &sampled.poses {
        if let Target::Entity { entity } = target {
            let e = out
                .entities
                .get_mut(*entity)
                .expect("validated entity track");
            let transform = pose.apply(e.transform.affine()?)?;
            e.transform = Transform {
                columns: transform.to_cols_array_2d(),
                operations: vec![],
            };
        }
    }
    let mut skeletons = BTreeMap::new();
    for (id, rig) in &state.rigs {
        let plan = rig.compile()?;
        let inverse = out.world_transform(*id)?.inverse();
        if !inverse.is_finite() {
            return Err(Error::new("singular_pose", "rig entity transform singular"));
        }
        let posed = plan.evaluate(
            *id,
            &sampled.poses,
            |target| Ok(inverse.transform_point3(out.world_transform(target)?.translation)),
            &mut cancelled,
        )?;
        skeletons.insert(*id, (plan, posed));
    }
    let entities: BTreeSet<_> = state
        .skins
        .keys()
        .chain(state.morphs.keys())
        .copied()
        .collect();
    let mut deformed_points = 0;
    for entity in entities {
        if cancelled() {
            return Err(Error::new("cancelled", "deformation cancelled"));
        }
        let mut mesh = entity_mesh(&out, entity)?.clone();
        let mut frames = state
            .shading_frames
            .get(&entity)
            .map(|f| crate::rigging::PreparedFrames::new(f, &mesh))
            .transpose()?;
        let mut positions: Vec<_> = (0..mesh.positions.len())
            .map(|i| mesh.positions.get(i))
            .collect();
        if let Some(morphs) = state.morphs.get(&entity) {
            for morph in morphs {
                let weight = sampled
                    .morphs
                    .get(&(entity, morph.id))
                    .copied()
                    .unwrap_or(morph.default_weight);
                if weight != 0. {
                    if let Some(f) = &mut frames {
                        f.morph(morph, weight, &mesh, &mut cancelled)?;
                    }
                    for (i, id) in mesh.point_ids.iter().enumerate() {
                        if let Some(delta) = morph.offsets.get(id) {
                            positions[i] += DVec3::from_array(*delta) * weight;
                        }
                    }
                }
            }
        }
        if let Some(skin) = state.skins.get(&entity) {
            let (plan, pose) = &skeletons[&skin.rig];
            let inverse = out.world_transform(entity)?.inverse();
            if !inverse.is_finite() {
                return Err(Error::new(
                    "singular_pose",
                    "skinned entity transform singular",
                ));
            }
            let rig_world = out.world_transform(skin.rig)?;
            let bind = skin.mesh_bind.affine()?;
            let matrices: BTreeMap<_, _> = plan
                .ids
                .iter()
                .enumerate()
                .map(|(i, id)| {
                    Ok((
                        *id,
                        inverse
                            * rig_world
                            * pose[i]
                            * skin
                                .inverse_binds
                                .get(id)
                                .map(Transform::affine)
                                .transpose()?
                                .unwrap_or(plan.inverse_bind[i])
                            * bind,
                    ))
                })
                .collect::<Result<_>>()?;
            let mut direction_matrices =
                frames.as_ref().map(|_| Vec::with_capacity(positions.len()));
            for (i, id) in mesh.point_ids.iter().enumerate() {
                if cancelled() {
                    return Err(Error::new("cancelled", "skinning cancelled"));
                }
                let mut point = DVec3::ZERO;
                for influence in &skin.weights[id] {
                    point += matrices[&influence.joint].transform_point3(positions[i])
                        * influence.weight;
                }
                positions[i] = point;
                if let Some(out) = &mut direction_matrices {
                    let mut linear = glam::DMat3::ZERO;
                    for influence in &skin.weights[id] {
                        linear += matrices[&influence.joint].matrix3 * influence.weight;
                    }
                    out.push(linear);
                }
            }
            if let (Some(f), Some(matrices)) = (&mut frames, direction_matrices) {
                f.skin(&matrices, &mesh, &mut cancelled)?;
            }
        }
        deformed_points += positions.len();
        mesh.positions =
            crate::geometry::Positions::F64(positions.into_iter().map(|p| p.to_array()).collect());
        // Flat geometric normals are the explicit deformation profile. Stale
        // imported shading frames must never be reused after morph/LBS changes.
        if let Some(f) = frames {
            f.write(&mut mesh)?;
        } else {
            mesh.attributes.retain(|_, a| {
                !matches!(a.semantic.as_str(), "normal" | "tangent" | "tangent_sign")
            });
        }
        mesh.validate()?;
        let key = mesh.content_id()?;
        out.meshes.insert(key.clone(), std::sync::Arc::new(mesh));
        out.entities
            .get_mut(entity)
            .expect("validated deformed entity")
            .mesh = Some(key);
        // UV constraints refer to authored mesh space, not disposable posed geometry.
        out.uv_bindings.remove(&entity);
    }
    // Poses and deformed meshes are disposable evaluated state, not rest state.
    out.animation = None;
    if !hidden.is_empty() {
        out.layers.push(Layer {
            name: "evaluated visibility".into(),
            overrides: hidden
                .into_iter()
                .map(|id| {
                    (
                        id,
                        Override {
                            transform: None,
                            material: None,
                            hidden: true,
                        },
                    )
                })
                .collect(),
        });
    }
    out.validate()?;
    Ok((out, deformed_points))
}
