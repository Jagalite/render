//! glTF animation adapter. Output consists solely of ordinary typed commands.
use super::*;
use crate::{animation as anim, rigging};

fn affine(a: DAffine3) -> Transform {
    Transform {
        columns: a.to_cols_array_2d(),
        operations: vec![],
    }
}
fn no_extensions(v: &Value) -> Result<()> {
    if !v.is_object() {
        return Err(bad("animation/skin records must be objects"));
    }
    if v.get("extensions").is_some() {
        return Err(unsupported("animation/skin extensions"));
    }
    Ok(())
}
fn seconds(v: f32) -> Result<Time> {
    if !v.is_finite() || !(0.0..=1e9).contains(&v) {
        return Err(bad("animation time outside [0,1e9] seconds"));
    }
    if v == 0. {
        return Time::new(0, 1);
    }
    let bits = v.to_bits();
    let exponent = ((bits >> 23) & 255) as i32 - 127 - 23;
    let mut numerator = u128::from((bits & 0x7fffff) | 0x800000);
    let mut exponent = exponent;
    while numerator.is_multiple_of(2) {
        numerator /= 2;
        exponent += 1;
    }
    if exponent < -63 {
        return Err(unsupported(
            "key time denominator exceeds exact rational profile",
        ));
    }
    if exponent >= 0 {
        numerator <<= exponent as u32;
    }
    anim::ratio(
        numerator as i128,
        if exponent < 0 { 1u128 << -exponent } else { 1 },
    )
}
fn uint(a: &Accessor<'_>, i: usize, axis: usize) -> Result<usize> {
    Ok(match a.component {
        5121 => a.component(i, axis)[0] as usize,
        5123 => u16::from_le_bytes(a.component(i, axis).try_into().expect("width")) as usize,
        _ => return Err(unsupported("JOINTS requires unsigned byte/short")),
    })
}
fn weights(a: &Accessor<'_>) -> Result<Vec<[f64; 4]>> {
    match (a.component, a.normalized) {
        (5126, false) => Ok(a
            .floats::<4>()?
            .into_iter()
            .map(|v| v.map(f64::from))
            .collect()),
        (5121 | 5123, true) => (0..a.count)
            .map(|i| {
                let mut out = [0.; 4];
                for (axis, w) in out.iter_mut().enumerate() {
                    *w = uint(a, i, axis)? as f64 / if a.component == 5121 { 255. } else { 65535. };
                }
                Ok(out)
            })
            .collect(),
        _ => Err(unsupported(
            "WEIGHTS requires float or normalized unsigned byte/short",
        )),
    }
}
fn base(node: &Value, path: &str) -> Result<anim::Value> {
    Ok(match path {
        "translation" => anim::Value::Vector(vector(node.get(path), [0.; 3])?),
        "scale" => anim::Value::Vector(vector(node.get(path), [1.; 3])?),
        "rotation" => anim::Value::Quaternion(
            DQuat::from_array(vector(node.get(path), [0., 0., 0., 1.])?)
                .normalize()
                .to_array(),
        ),
        _ => return Err(bad("unknown transform path")),
    })
}
fn property(path: &str) -> Result<anim::Property> {
    Ok(match path {
        "translation" => anim::Property::AbsoluteTranslation,
        "rotation" => anim::Property::AbsoluteQuaternion,
        "scale" => anim::Property::AbsoluteScale,
        "weights" => anim::Property::MorphWeight,
        _ => return Err(unsupported("animation target path")),
    })
}

pub(super) fn append(
    root: &Value,
    buffers: &[Vec<u8>],
    document: Id,
    commands: &mut Vec<Command>,
    report: &mut Report,
) -> Result<()> {
    let nodes = array(root, "nodes")?;
    let animations = array(root, "animations")?;
    let skins = array(root, "skins")?;
    let meshes = array(root, "meshes")?;
    let dynamic = !animations.is_empty()
        || !skins.is_empty()
        || meshes.iter().any(|m| {
            m.get("weights").is_some()
                || array(m, "primitives")
                    .is_ok_and(|p| p.iter().any(|p| p.get("targets").is_some()))
        })
        || nodes
            .iter()
            .any(|n| n.get("weights").is_some() || n.get("skin").is_some());
    if !dynamic {
        if meshes.iter().any(|m| {
            array(m, "primitives").is_ok_and(|p| {
                p.iter().any(|p| {
                    p["attributes"].get("JOINTS_0").is_some()
                        || p["attributes"].get("WEIGHTS_0").is_some()
                        || p["attributes"].get("JOINTS_1").is_some()
                        || p["attributes"].get("WEIGHTS_1").is_some()
                })
            })
        }) {
            return Err(unsupported("skin attributes without skin binding"));
        }
        return Ok(());
    }
    if animations.len() > 32 || skins.len() > 32 {
        return Err(Error::new("budget", "32 clips/skins per import"));
    }
    let id = |kind: &str, n: usize| identity(document, &report.source_digest, kind, n);
    let entities: BTreeMap<_, _> = commands
        .iter()
        .filter_map(|c| {
            if let Command::CreateEntity { entity } = c {
                Some((entity.id, entity.clone()))
            } else {
                None
            }
        })
        .collect();
    let assets: BTreeMap<_, _> = commands
        .iter()
        .filter_map(|c| {
            if let Command::PutMesh { mesh } = c {
                Some(mesh.content_id().map(|id| (id, mesh.clone())))
            } else {
                None
            }
        })
        .collect::<Result<_>>()?;
    let mut globals = BTreeMap::new();
    while globals.len() < entities.len() {
        let before = globals.len();
        for e in entities.values() {
            if globals.contains_key(&e.id) || e.parent.is_some_and(|p| !globals.contains_key(&p)) {
                continue;
            }
            let local = e.transform.affine()?;
            globals.insert(e.id, e.parent.map_or(local, |p| globals[&p] * local));
        }
        if before == globals.len() {
            return Err(bad("invalid imported hierarchy"));
        }
    }
    let mut state = anim::State::default();
    let rig_id = id("animation-rig", 0)?;
    if !skins.is_empty() {
        let mut joints = vec![];
        for entity in report.source_nodes.values() {
            let e = &entities[entity];
            let inverse = globals[entity].inverse();
            if !inverse.is_finite() {
                return Err(unsupported("singular skeleton rest transform"));
            }
            joints.push(rigging::Joint {
                id: *entity,
                parent: e.parent,
                rest: e.transform.clone(),
                inverse_bind: affine(inverse),
            });
        }
        let rig = rigging::Rig {
            joints,
            constraints: vec![],
        };
        rig.compile()?;
        state.rigs.insert(rig_id, rig);
        commands.push(Command::CreateEntity {
            entity: Entity {
                id: rig_id,
                name: "Imported skeleton".into(),
                parent: None,
                mesh: None,
                material: None,
                transform: Transform::default(),
            },
        });
    }
    let mut palettes = vec![];
    for (si, skin) in skins.iter().enumerate() {
        no_extensions(skin)?;
        let source_joints = array(skin, "joints")?;
        if source_joints.is_empty() || source_joints.len() > 64 {
            return Err(bad("skin needs 1..64 joints"));
        }
        let joints: Vec<_> = source_joints
            .iter()
            .map(|v| {
                report
                    .source_nodes
                    .get(&index(v)?)
                    .copied()
                    .ok_or_else(|| unsupported("skin joint outside selected scene"))
            })
            .collect::<Result<_>>()?;
        if joints.iter().collect::<BTreeSet<_>>().len() != joints.len() {
            return Err(bad("duplicate skin joint"));
        }
        let roots: BTreeSet<_> = joints
            .iter()
            .map(|joint| {
                let mut root = *joint;
                while let Some(parent) = entities[&root].parent {
                    root = parent;
                }
                root
            })
            .collect();
        if roots.len() != 1 {
            return Err(bad("skin joints must share a common root"));
        }
        if let Some(skeleton) = skin.get("skeleton") {
            let skeleton = *report
                .source_nodes
                .get(&index(skeleton)?)
                .ok_or_else(|| bad("skeleton outside selected scene"))?;
            for joint in &joints {
                let mut ancestor = Some(*joint);
                while ancestor.is_some() && ancestor != Some(skeleton) {
                    ancestor = entities[&ancestor.expect("some")].parent;
                }
                if ancestor.is_none() {
                    return Err(bad("skeleton must be a common ancestor of joints"));
                }
            }
        }
        let matrices = if let Some(a) = skin.get("inverseBindMatrices") {
            let a = accessor(root, buffers, index(a)?, "MAT4", 16)?;
            if a.count != joints.len() {
                return Err(bad("inverse bind matrix count"));
            }
            a.floats::<16>()?
                .into_iter()
                .map(|m| {
                    let m = m.map(f64::from);
                    if [m[3], m[7], m[11], m[15]] != [0., 0., 0., 1.] {
                        return Err(bad("non-affine inverse bind"));
                    }
                    let t = affine(DAffine3::from_mat4(DMat4::from_cols_array(&m)));
                    t.inverse()?;
                    Ok(t)
                })
                .collect::<Result<Vec<_>>>()?
        } else {
            vec![Transform::default(); joints.len()]
        };
        palettes.push((joints, matrices));
        report.source_skins.insert(si, rig_id);
    }
    let mut morph_ids = BTreeMap::<usize, Vec<(Id, Vec<Id>)>>::new();
    let mut total_offsets = 0usize;
    let mut bound_skins = BTreeSet::new();
    for &ni in report.source_nodes.keys() {
        let node = &nodes[ni];
        let Some(mi) = node.get("mesh") else {
            if node.get("weights").is_some() || node.get("skin").is_some() {
                return Err(bad("skin/weights node requires mesh"));
            }
            continue;
        };
        let mesh = at(root, "meshes", index(mi)?)?;
        let parts = array(mesh, "primitives")?;
        let count = array(&parts[0], "targets")?.len();
        if count > 64 {
            return Err(Error::new("budget", "64 morph targets per primitive"));
        }
        for weights in [mesh.get("weights"), node.get("weights")]
            .into_iter()
            .flatten()
        {
            if weights.as_array().is_none_or(|w| w.len() != count) {
                return Err(bad("default morph weight count"));
            }
            for w in weights.as_array().expect("checked") {
                let w = number(w)?;
                if !(-8.0..=8.).contains(&w) {
                    return Err(unsupported("morph weight outside [-8,8]"));
                }
            }
        }
        let defaults = node.get("weights").or_else(|| mesh.get("weights"));
        for (pi, p) in parts.iter().enumerate() {
            let targets = array(p, "targets")?;
            if targets.len() != count {
                return Err(bad("mesh primitives must share morph target count"));
            }
            let child = id(&format!("node:{ni}:primitive"), pi)?;
            let geometry = &assets[entities[&child].mesh.as_ref().expect("mesh")];
            let topology = crate::groom::topology(geometry)?;
            let mut morphs = vec![];
            let mut mids = vec![];
            for (ti, target) in targets.iter().enumerate() {
                let map = target
                    .as_object()
                    .ok_or_else(|| bad("morph target must be object"))?;
                if map.len() != 1 || !map.contains_key("POSITION") {
                    return Err(unsupported(
                        "morph profile requires POSITION only; normal/tangent morphs unsupported",
                    ));
                }
                let a = accessor(root, buffers, index(&target["POSITION"])?, "VEC3", 3)?;
                if a.count != geometry.positions.len() {
                    return Err(bad("morph point count"));
                }
                total_offsets += a.count;
                if total_offsets > 131072 {
                    return Err(Error::new("budget", "131072 imported morph offsets"));
                }
                let mid = id(&format!("node:{ni}:primitive:{pi}:morph"), ti)?;
                morphs.push(rigging::Morph {
                    id: mid,
                    topology: topology.clone(),
                    default_weight: defaults.map(|v| number(&v[ti])).transpose()?.unwrap_or(0.),
                    offsets: geometry
                        .point_ids
                        .iter()
                        .copied()
                        .zip(a.floats::<3>()?.into_iter().map(|v| v.map(f64::from)))
                        .collect(),
                });
                mids.push(mid);
            }
            if !morphs.is_empty() {
                state.morphs.insert(child, morphs);
                morph_ids.entry(ni).or_default().push((child, mids));
            }
            let attributes = &p["attributes"];
            if let Some(si) = node.get("skin") {
                let si = index(si)?;
                let (joints, binds) = palettes.get(si).ok_or_else(|| bad("skin index"))?;
                bound_skins.insert(si);
                let mut influences = vec![BTreeMap::<Id, f64>::new(); geometry.positions.len()];
                for set in 0..2 {
                    let ja = attributes.get(format!("JOINTS_{set}"));
                    let wa = attributes.get(format!("WEIGHTS_{set}"));
                    if set == 1 && ja.is_none() && wa.is_none() {
                        continue;
                    }
                    let ja = accessor(
                        root,
                        buffers,
                        index(ja.ok_or_else(|| bad("paired JOINTS/WEIGHTS required"))?)?,
                        "VEC4",
                        4,
                    )?;
                    let wa = accessor_mode(
                        root,
                        buffers,
                        index(wa.ok_or_else(|| bad("paired JOINTS/WEIGHTS required"))?)?,
                        "VEC4",
                        4,
                        true,
                    )?;
                    if ja.count != influences.len() || wa.count != influences.len() {
                        return Err(bad("skin attribute point count"));
                    }
                    let weights = weights(&wa)?;
                    for (point, accum) in influences.iter_mut().enumerate() {
                        for (axis, &w) in weights[point].iter().enumerate() {
                            let joint = *joints
                                .get(uint(&ja, point, axis)?)
                                .ok_or_else(|| bad("joint palette index"))?;
                            if !(0.0..=1.).contains(&w) {
                                return Err(bad("skin weight range"));
                            }
                            if w > 0. {
                                *accum.entry(joint).or_default() += w;
                            }
                        }
                    }
                }
                let weights = geometry
                    .point_ids
                    .iter()
                    .copied()
                    .zip(influences)
                    .map(|(point, accum)| {
                        let sum: f64 = accum.values().sum();
                        if sum <= 0. || (sum - 1.).abs() > 0.01 {
                            return Err(bad("skin weight sum differs from one by more than 0.01"));
                        }
                        Ok((
                            point,
                            accum
                                .into_iter()
                                .map(|(joint, w)| rigging::Influence {
                                    joint,
                                    weight: w / sum,
                                })
                                .collect(),
                        ))
                    })
                    .collect::<Result<_>>()?;
                let skin = rigging::Skin {
                    rig: rig_id,
                    topology: topology.clone(),
                    mesh_bind: Transform::default(),
                    inverse_binds: joints.iter().copied().zip(binds.iter().cloned()).collect(),
                    weights,
                };
                skin.validate(geometry, &state.rigs[&rig_id])?;
                state.skins.insert(child, skin);
            } else if ["JOINTS_0", "WEIGHTS_0", "JOINTS_1", "WEIGHTS_1"]
                .iter()
                .any(|k| attributes.get(k).is_some())
            {
                return Err(unsupported("skin attributes on an unskinned instance"));
            }
        }
    }
    if bound_skins.len() != skins.len() {
        return Err(unsupported("unreferenced skin outside selected scene"));
    }
    let mut total_keys = 0usize;
    let mut quaternion_error = 0f64;
    for (ai, animation) in animations.iter().enumerate() {
        no_extensions(animation)?;
        let channels = array(animation, "channels")?;
        let samplers = array(animation, "samplers")?;
        if channels.is_empty() || channels.len() > 256 || samplers.len() > 256 {
            return Err(Error::new(
                "budget",
                "animation requires 1..256 channels and at most 256 samplers",
            ));
        }
        let mut tracks = vec![];
        let mut used = BTreeSet::new();
        let mut transform_nodes = BTreeSet::new();
        let mut start = None::<Time>;
        let mut end = Time::new(0, 1)?;
        for channel in channels {
            no_extensions(channel)?;
            no_extensions(&channel["target"])?;
            let ni = index(&channel["target"]["node"])?;
            let entity = *report
                .source_nodes
                .get(&ni)
                .ok_or_else(|| unsupported("animation target outside selected scene"))?;
            let path = channel["target"]["path"]
                .as_str()
                .ok_or_else(|| bad("animation path"))?;
            let property = property(path)?;
            if !used.insert((ni, path.to_string())) {
                return Err(bad("duplicate animation target/path"));
            }
            if path != "weights" {
                if nodes[ni].get("matrix").is_some() {
                    return Err(bad("animated node cannot use matrix"));
                }
                transform_nodes.insert(ni);
            }
            let sampler = samplers
                .get(index(&channel["sampler"])?)
                .ok_or_else(|| bad("animation sampler index"))?;
            no_extensions(sampler)?;
            let interpolation = match sampler
                .get("interpolation")
                .and_then(Value::as_str)
                .unwrap_or("LINEAR")
            {
                "STEP" => anim::Interpolation::Step,
                "LINEAR" => anim::Interpolation::Linear,
                "CUBICSPLINE" => anim::Interpolation::Cubic,
                _ => return Err(unsupported("animation interpolation")),
            };
            if sampler.get("interpolation").is_some_and(|v| !v.is_string()) {
                return Err(bad("interpolation must be string"));
            }
            let input = accessor(root, buffers, index(&sampler["input"])?, "SCALAR", 1)?;
            if input.count > 16384 {
                return Err(Error::new("budget", "16384 keys per channel"));
            }
            let times = input
                .floats::<1>()?
                .into_iter()
                .map(|v| seconds(v[0]))
                .collect::<Result<Vec<_>>>()?;
            for pair in times.windows(2) {
                if pair[0].compare(pair[1])? != std::cmp::Ordering::Less {
                    return Err(bad("key times must strictly increase"));
                }
            }
            if start.is_none_or(|s| times[0].compare(s).is_ok_and(|c| c.is_lt())) {
                start = Some(times[0]);
            }
            if times.last().expect("nonempty").compare(end)?.is_gt() {
                end = *times.last().expect("nonempty");
            }
            let copies = if path == "weights" {
                morph_ids
                    .get(&ni)
                    .ok_or_else(|| bad("weights channel requires morph targets"))?
                    .iter()
                    .map(|(_, ids)| ids.len())
                    .sum()
            } else if skins.is_empty() {
                1
            } else {
                2
            };
            total_keys = total_keys
                .checked_add(times.len() * copies)
                .ok_or_else(|| Error::new("budget", "key count overflow"))?;
            if total_keys > 131072 {
                return Err(Error::new("budget", "131072 expanded import keys"));
            }
            let factor = if interpolation == anim::Interpolation::Cubic {
                3
            } else {
                1
            };
            let width = if path == "weights" {
                morph_ids[&ni][0].1.len()
            } else {
                1
            };
            let (shape, axes) = match path {
                "rotation" => ("VEC4", 4),
                "weights" => ("SCALAR", 1),
                _ => ("VEC3", 3),
            };
            let output = accessor(root, buffers, index(&sampler["output"])?, shape, axes)?;
            if output.count != times.len() * factor * width {
                return Err(bad("animation output count"));
            }
            let values: Vec<anim::Value> = match axes {
                4 => output
                    .floats::<4>()?
                    .into_iter()
                    .enumerate()
                    .map(|(i, v)| {
                        let q = DQuat::from_array(v.map(f64::from));
                        let is_key = factor == 1 || i % 3 == 1;
                        if is_key {
                            quaternion_error =
                                quaternion_error.max((q.length_squared() - 1.).abs());
                        }
                        if is_key && (q.length_squared() - 1.).abs() > 1e-3 {
                            return Err(bad("rotation key must be unit quaternion"));
                        }
                        Ok(anim::Value::Quaternion(if is_key {
                            q.normalize().to_array()
                        } else {
                            q.to_array()
                        }))
                    })
                    .collect::<Result<_>>()?,
                3 => output
                    .floats::<3>()?
                    .into_iter()
                    .map(|v| anim::Value::Vector(v.map(f64::from)))
                    .collect(),
                _ => output
                    .floats::<1>()?
                    .into_iter()
                    .map(|v| anim::Value::Scalar(f64::from(v[0])))
                    .collect(),
            };
            if path == "weights" {
                for (i, value) in values.iter().enumerate() {
                    let is_key = factor == 1 || (i / width) % 3 == 1;
                    if is_key && matches!(value, anim::Value::Scalar(w) if !(-8.0..=8.).contains(w))
                    {
                        return Err(unsupported("morph key weight outside [-8,8]"));
                    }
                }
            }
            let destinations: Vec<_> = if path == "weights" {
                morph_ids[&ni]
                    .iter()
                    .flat_map(|(entity, ids)| {
                        ids.iter().enumerate().map(|(i, target)| {
                            (
                                anim::Target::Morph {
                                    entity: *entity,
                                    target: *target,
                                },
                                i,
                            )
                        })
                    })
                    .collect()
            } else {
                let mut targets = vec![(anim::Target::Entity { entity }, 0)];
                if !skins.is_empty() {
                    targets.push((
                        anim::Target::Joint {
                            rig: rig_id,
                            joint: entity,
                        },
                        0,
                    ));
                }
                targets
            };
            for (target, component) in destinations {
                let keys = times
                    .iter()
                    .enumerate()
                    .map(|(i, time)| {
                        let at = i * factor * width + component;
                        anim::Key {
                            time: *time,
                            value: values[at + if factor == 3 { width } else { 0 }],
                            incoming: if factor == 3 { Some(values[at]) } else { None },
                            outgoing: if factor == 3 {
                                Some(values[at + 2 * width])
                            } else {
                                None
                            },
                        }
                    })
                    .collect();
                tracks.push(anim::Track {
                    id: id(&format!("clip:{ai}:track"), tracks.len())?,
                    target,
                    property,
                    interpolation,
                    keys,
                });
            }
        }
        let start = start.expect("channels");
        if start == end {
            end = start.add_time(Time::new(1, 1)?)?;
        }
        for ni in transform_nodes {
            for path in ["translation", "rotation", "scale"] {
                if used.contains(&(ni, path.into())) {
                    continue;
                }
                let entity = report.source_nodes[&ni];
                let mut targets = vec![anim::Target::Entity { entity }];
                if !skins.is_empty() {
                    targets.push(anim::Target::Joint {
                        rig: rig_id,
                        joint: entity,
                    });
                }
                for target in targets {
                    total_keys += 1;
                    if total_keys > 131072 {
                        return Err(Error::new("budget", "131072 expanded import keys"));
                    }
                    tracks.push(anim::Track {
                        id: id(&format!("clip:{ai}:track"), tracks.len())?,
                        target,
                        property: property(path)?,
                        interpolation: anim::Interpolation::Step,
                        keys: vec![anim::Key {
                            time: start,
                            value: base(&nodes[ni], path)?,
                            incoming: None,
                            outgoing: None,
                        }],
                    });
                }
            }
        }
        let clip_id = id("clip", ai)?;
        let clip = anim::Clip {
            id: clip_id,
            start,
            end,
            extrapolation: anim::Extrapolation::Clamp,
            remap: anim::TimeMap {
                rate: Time::new(1, 1)?,
                offset: Time::new(0, 1)?,
            },
            tracks,
        };
        clip.validate()?;
        state.clips.insert(clip_id, clip);
        report.source_clips.insert(ai, clip_id);
    }
    for morphs in state.morphs.values() {
        for morph in morphs {
            if !(-8.0..=8.).contains(&morph.default_weight) {
                return Err(unsupported("morph default weight outside [-8,8]"));
            }
        }
    }
    if commands.len() + 1 > 256 {
        return Err(Error::new("budget", "import exceeds 256 commands"));
    }
    commands.push(Command::MergeAnimation { animation: state });
    report.profile = "gltf2-animated-pbr-v0".into();
    report.losses.push("Animation and skin names remain in the original source; native IDs and complete converted channels/bindings are retained. Imported node and rig tracks are separate native authoring components.".into());
    report.losses.push(format!(
        "Maximum source quaternion squared-norm correction: {quaternion_error:.9e}"
    ));
    report.losses.push("Absolute TRS clips; float32 key times retained exactly within rational limits; unit rotation keys renormalized within 1e-3 squared-norm error. Skin weights normalized within 0.01 of unity, duplicate influences combined. Deformation recomputes flat geometric normals; POSITION-only morphs; no animation/skin glTF export. Single-time clips use a one-second constant interval.".into());
    Ok(())
}
