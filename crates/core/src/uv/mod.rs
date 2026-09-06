//! Bounded native UV charts, authored constraints and atlas operations.
mod authoring;
pub use authoring::{Asset, Operation, Prepared, Request, prepare};
mod charts;
mod packing;
pub use packing::{ChartTransform, Pack, PackingReport, PinPacking, pack};
mod solver;
use crate::{Error, Id, Result, canonical, digest, geometry::*};
use glam::{DVec2, DVec3};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
pub const PROFILE: &str = "uv-lscm-disk-v1";
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pin {
    pub corner: u64,
    pub uv: [f32; 2],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unwrap {
    pub attribute: String,
    pub attribute_id: Id,
    pub seams: BTreeSet<u64>,
    pub pins: Vec<Pin>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Budget {
    pub max_output_bytes: u64,
    pub max_solver_work: u64,
}
impl Default for Budget {
    fn default() -> Self {
        Self {
            max_output_bytes: 32 * 1024 * 1024,
            max_solver_work: 16_777_216,
        }
    }
}
impl Budget {
    fn validate(&self) -> Result<()> {
        if self.max_output_bytes == 0
            || self.max_output_bytes > 32 * 1024 * 1024
            || self.max_solver_work == 0
            || self.max_solver_work > 16_777_216
        {
            return Err(Error::new("budget", "UV budget exceeds bounded profile"));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Atlas {
    pub width: u32,
    pub height: u32,
    pub padding_pixels: u32,
    pub pixels_per_meter: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    pub version: u32,
    pub mesh: String,
    pub attribute: String,
    pub attribute_id: Id,
    pub seams: BTreeSet<u64>,
    pub pins: Vec<Pin>,
    pub atlas: Option<Atlas>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChartReport {
    pub anchor_face: u64,
    pub faces: Vec<u64>,
    pub corners: Vec<u64>,
    pub vertices: u32,
    pub triangles: u32,
    pub boundary_vertices: u32,
    pub surface_area_square_meters: f64,
    pub uv_area: f64,
    pub bounds: [[f64; 2]; 2],
    pub least_squares_residual: Option<f64>,
    pub qr_pivot_ratio: Option<f64>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub profile: String,
    pub source_mesh: String,
    pub output_mesh: String,
    pub layout: String,
    pub output_bytes: u64,
    pub solver_work: u64,
    pub packing: Option<PackingReport>,
    pub charts: Vec<ChartReport>,
}
pub struct Computed {
    pub mesh: Mesh,
    pub layout: Layout,
    pub report: Report,
}
fn check(cancel: &mut impl FnMut() -> bool) -> Result<()> {
    if cancel() {
        Err(Error::new(
            "cancelled",
            "UV work cancelled before publication",
        ))
    } else {
        Ok(())
    }
}
fn bad(message: &str) -> Error {
    Error::new("uv", message)
}
fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > 64 || name.chars().any(char::is_control) {
        return Err(bad(
            "UV attribute name must be 1..64 bytes without controls",
        ));
    }
    Ok(())
}
fn values<'a>(mesh: &'a Mesh, name: &str, id: Id) -> Result<&'a [[f32; 2]]> {
    let a = mesh
        .attributes
        .get(name)
        .ok_or_else(|| Error::new("reference", "UV attribute missing"))?;
    if a.id != id || a.domain != Domain::Corner || a.semantic != "uv" {
        return Err(Error::new(
            "reference",
            "UV attribute identity/domain mismatch",
        ));
    }
    let AttributeValues::Vec2(v) = &a.values else {
        return Err(bad("UV attribute must contain vec2"));
    };
    Ok(v)
}
fn pin_indices(mesh: &Mesh, pins: &[Pin]) -> Result<BTreeMap<usize, [f32; 2]>> {
    let by_id: BTreeMap<_, _> = mesh
        .corner_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (*id, i))
        .collect();
    let mut result = BTreeMap::new();
    for pin in pins {
        if pin.uv.iter().any(|v| !v.is_finite() || v.abs() > 1024.) {
            return Err(bad("UV pin exceeds finite coordinate profile"));
        }
        let c = *by_id
            .get(&pin.corner)
            .ok_or_else(|| Error::new("stale_selection", "UV pin corner is absent"))?;
        if result.insert(c, pin.uv).is_some() {
            return Err(bad("duplicate UV pin corner"));
        }
    }
    Ok(result)
}
fn position(mesh: &Mesh, chart: &charts::Chart, v: usize) -> DVec3 {
    mesh.positions
        .get(mesh.corners[chart.vertices[v]].vertex as usize)
}
fn orient(a: DVec2, b: DVec2, c: DVec2) -> f64 {
    (b - a).perp_dot(c - a)
}
fn inside(p: DVec2, t: [DVec2; 3], eps: f64) -> bool {
    (0..3).all(|i| orient(t[i], t[(i + 1) % 3], p) > eps)
}
fn triangle_overlap(a: [DVec2; 3], b: [DVec2; 3], eps: f64) -> bool {
    if a.iter().any(|&p| inside(p, b, eps))
        || b.iter().any(|&p| inside(p, a, eps))
        || inside((a[0] + a[1] + a[2]) / 3., b, eps)
        || inside((b[0] + b[1] + b[2]) / 3., a, eps)
    {
        return true;
    }
    for i in 0..3 {
        for j in 0..3 {
            let x = orient(a[i], a[(i + 1) % 3], b[j]);
            let y = orient(a[i], a[(i + 1) % 3], b[(j + 1) % 3]);
            let z = orient(b[j], b[(j + 1) % 3], a[i]);
            let w = orient(b[j], b[(j + 1) % 3], a[(i + 1) % 3]);
            if ((x > eps && y < -eps) || (y > eps && x < -eps))
                && ((z > eps && w < -eps) || (w > eps && z < -eps))
            {
                return true;
            }
        }
    }
    false
}
fn measure(
    mesh: &Mesh,
    chart: &charts::Chart,
    uv: &[[f32; 2]],
    cancel: &mut impl FnMut() -> bool,
) -> Result<ChartReport> {
    let mut lo = DVec2::splat(f64::INFINITY);
    let mut hi = DVec2::splat(f64::NEG_INFINITY);
    for &c in chart.corners.keys() {
        let p = DVec2::from_array(uv[c].map(f64::from));
        if !p.is_finite() || p.abs().max_element() > 1024. {
            return Err(bad("UV coordinates exceed finite profile"));
        }
        lo = lo.min(p);
        hi = hi.max(p);
    }
    let scale = (hi - lo).max_element();
    if scale <= 0. {
        return Err(bad("UV chart collapsed"));
    }
    let eps = scale * scale * 1e-10;
    let mut surface_area = 0.;
    let mut uv_area = 0.;
    let mut triangles = Vec::new();
    for t in &chart.triangles {
        check(cancel)?;
        let p = t.map(|v| position(mesh, chart, v));
        let coords = t.map(|v| DVec2::from_array(uv[chart.vertices[v]].map(f64::from)));
        let area = orient(coords[0], coords[1], coords[2]) / 2.;
        if area <= eps {
            return Err(Error::new(
                "uv_fold",
                "UV triangle flipped or collapsed after f32 storage",
            ));
        }
        uv_area += area;
        surface_area += (p[1] - p[0]).cross(p[2] - p[0]).length() / 2.;
        triangles.push(coords);
    }
    if !surface_area.is_finite() || surface_area <= 0. {
        return Err(bad("invalid UV source area"));
    }
    for i in 0..triangles.len() {
        for j in i + 1..triangles.len() {
            check(cancel)?;
            if triangle_overlap(triangles[i], triangles[j], eps) {
                return Err(Error::new("uv_overlap", "UV chart triangles overlap"));
            }
        }
    }
    // Positive triangle areas alone do not exclude a chart boundary touching
    // itself. Permit incidence only through the same chart vertex identity.
    let boundary: Vec<_> = chart
        .boundary
        .iter()
        .map(|&v| DVec2::from_array(uv[chart.vertices[v]].map(f64::from)))
        .collect();
    let on_segment =
        |p: DVec2, a: DVec2, b: DVec2| orient(a, b, p).abs() <= eps && (p - a).dot(p - b) <= eps;
    for i in 0..boundary.len() {
        let next_i = (i + 1) % boundary.len();
        for j in i + 1..boundary.len() {
            check(cancel)?;
            let next_j = (j + 1) % boundary.len();
            if next_i == j || next_j == i {
                continue;
            }
            let (a, b, c, d) = (boundary[i], boundary[next_i], boundary[j], boundary[next_j]);
            if on_segment(a, c, d)
                || on_segment(b, c, d)
                || on_segment(c, a, b)
                || on_segment(d, a, b)
            {
                return Err(Error::new(
                    "uv_overlap",
                    "nonincident UV chart boundary edges touch",
                ));
            }
        }
    }
    let mut corners: Vec<_> = chart.corners.keys().map(|&c| mesh.corner_ids[c]).collect();
    corners.sort_unstable();
    Ok(ChartReport {
        anchor_face: mesh.face_ids[chart.faces[0]],
        faces: chart.faces.iter().map(|&f| mesh.face_ids[f]).collect(),
        corners,
        vertices: chart.vertices.len() as u32,
        triangles: chart.triangles.len() as u32,
        boundary_vertices: chart.boundary.len() as u32,
        surface_area_square_meters: surface_area,
        uv_area,
        bounds: [lo.to_array(), hi.to_array()],
        least_squares_residual: None,
        qr_pivot_ratio: None,
    })
}
impl Layout {
    pub fn content_id(&self) -> Result<String> {
        Ok(digest(&canonical(self)?))
    }
    pub fn validate(&self, mesh: &Mesh) -> Result<()> {
        if self.version != 0 {
            return Err(Error::new(
                "schema_version",
                "UV layout version unsupported",
            ));
        }
        validate_name(&self.attribute)?;
        if mesh.content_id()? != self.mesh {
            return Err(Error::new("stale_selection", "UV layout mesh differs"));
        }
        let uv = values(mesh, &self.attribute, self.attribute_id)?;
        let pins = pin_indices(mesh, &self.pins)?;
        for (&c, &p) in &pins {
            if uv[c] != p {
                return Err(bad(
                    "UV layout pin moved without updating its authored constraint",
                ));
            }
        }
        let charts = charts::analyze(mesh, &self.seams, || false)?;
        let mut reports = Vec::new();
        for chart in &charts {
            for (&c, &v) in &chart.corners {
                if uv[c] != uv[chart.vertices[v]] {
                    return Err(bad("UV discontinuity requires an authored seam"));
                }
            }
            reports.push(measure(mesh, chart, uv, &mut || false)?);
        }
        if let Some(atlas) = &self.atlas {
            packing::validate(atlas, &reports)?;
        }
        Ok(())
    }
}
pub fn unwrap(
    mesh: &Mesh,
    request: &Unwrap,
    budget: &Budget,
    mut cancel: impl FnMut() -> bool,
) -> Result<Computed> {
    check(&mut cancel)?;
    budget.validate()?;
    validate_name(&request.attribute)?;
    if mesh
        .attributes
        .values()
        .any(|a| matches!(a.semantic.as_str(), "tangent" | "tangent_sign"))
    {
        return Err(Error::new(
            "uv_frame",
            "UV authoring requires generated tangent frames; explicitly resolve authored tangent data first",
        ));
    }
    if let Some(existing) = mesh.attributes.get(&request.attribute)
        && (existing.id != request.attribute_id || existing.semantic != "uv")
    {
        return Err(bad("existing UV name has different identity or semantic"));
    }
    if mesh
        .attributes
        .iter()
        .any(|(name, a)| name != &request.attribute && a.id == request.attribute_id)
    {
        return Err(bad("UV attribute identity already belongs to another name"));
    }
    let charts = charts::analyze(mesh, &request.seams, &mut cancel)?;
    let pins = pin_indices(mesh, &request.pins)?;
    let source = mesh.content_id()?;
    let mut output = mesh.clone();
    output.default_uv_attribute = Some(mesh.default_uv_id().unwrap_or(request.attribute_id));
    let mut uv = vec![[0.; 2]; mesh.corners.len()];
    let mut reports = Vec::new();
    let mut work = 0u64;
    for chart in &charts {
        check(&mut cancel)?;
        let n = chart.vertices.len();
        let mut fixed = BTreeMap::<usize, [f64; 2]>::new();
        for (&c, &value) in &pins {
            if let Some(&v) = chart.corners.get(&c) {
                let p = value.map(f64::from);
                if fixed.insert(v, p).is_some_and(|old| old != p) {
                    return Err(bad("pins disagree on one chart vertex"));
                }
            }
        }
        if fixed.is_empty() {
            let mut pair = (0, 1);
            let mut distance = 0f64;
            for i in 0..n {
                for j in i + 1..n {
                    let d = (position(mesh, chart, i) - position(mesh, chart, j)).length_squared();
                    if d > distance {
                        pair = (i, j);
                        distance = d;
                    }
                }
            }
            if !distance.is_finite() || distance <= 0. {
                return Err(bad("cannot anchor UV chart"));
            }
            fixed.insert(pair.0, [0., 0.]);
            fixed.insert(pair.1, [1., 0.]);
        }
        if fixed.len() < 2 || fixed.values().all(|p| p == fixed.values().next().unwrap()) {
            return Err(bad("each pinned chart needs two distinct UV pin positions"));
        }
        let free: Vec<_> = (0..n).filter(|v| !fixed.contains_key(v)).collect();
        let indices: BTreeMap<_, _> = free.iter().enumerate().map(|(i, v)| (*v, i)).collect();
        let columns = free.len() * 2;
        let rows = chart.triangles.len() * 2;
        let cost = (rows as u64) * (columns as u64) * (columns as u64);
        work = work
            .checked_add(cost)
            .ok_or_else(|| Error::new("budget", "UV solver work overflow"))?;
        if work > budget.max_solver_work {
            return Err(Error::new("budget", "UV solver work budget exceeded"));
        }
        let origin = position(mesh, chart, 0);
        let scale = (0..n)
            .map(|v| (position(mesh, chart, v) - origin).length())
            .fold(0f64, f64::max);
        if !scale.is_finite() || scale <= 0. {
            return Err(bad("invalid UV chart metric scale"));
        }
        let mut a = vec![vec![0.; columns]; rows];
        let mut b = vec![0.; rows];
        for (f, t) in chart.triangles.iter().enumerate() {
            check(&mut cancel)?;
            let p = t.map(|v| (position(mesh, chart, v) - origin) / scale);
            let e = p[1] - p[0];
            let length = e.length();
            let x = (p[2] - p[0]).dot(e) / length;
            let y = e.cross(p[2] - p[0]).length() / length;
            let area2 = length * y;
            if !area2.is_finite() || area2 <= 1e-12 {
                return Err(Error::new(
                    "uv_numeric",
                    "UV triangle metric conditioning exceeds profile",
                ));
            }
            let weights =
                [[x - length, y], [-x, -y], [length, 0.]].map(|v| v.map(|x| x / area2.sqrt()));
            for (j, &v) in t.iter().enumerate() {
                let [real, imag] = weights[j];
                if let Some(p) = fixed.get(&v) {
                    b[2 * f] -= real * p[0] - imag * p[1];
                    b[2 * f + 1] -= imag * p[0] + real * p[1];
                } else {
                    let k = indices[&v];
                    a[2 * f][k] += real;
                    a[2 * f][k + free.len()] -= imag;
                    a[2 * f + 1][k] += imag;
                    a[2 * f + 1][k + free.len()] += real;
                }
            }
        }
        let solved = if columns > 0 {
            Some(solver::solve(&a, &b, &mut cancel)?)
        } else {
            None
        };
        for (&c, &v) in &chart.corners {
            let p = if let Some(p) = fixed.get(&v) {
                *p
            } else {
                let k = indices[&v];
                let s = solved.as_ref().unwrap();
                [s.x[k], s.x[k + free.len()]]
            };
            uv[c] = p.map(|x| x as f32);
        }
        let mut report = measure(mesh, chart, &uv, &mut cancel)?;
        report.least_squares_residual = Some(b.iter().map(|v| v * v).sum::<f64>().sqrt());
        if let Some(s) = solved {
            report.least_squares_residual = Some(s.residual);
            report.qr_pivot_ratio = Some(s.pivot_ratio);
        }
        reports.push(report);
    }
    output.attributes.insert(
        request.attribute.clone(),
        Attribute {
            id: request.attribute_id,
            domain: Domain::Corner,
            semantic: "uv".into(),
            transfer: Transfer::Linear,
            values: AttributeValues::Vec2(uv),
        },
    );
    output.validate()?;
    let key = output.content_id()?;
    let layout = Layout {
        version: 0,
        mesh: key.clone(),
        attribute: request.attribute.clone(),
        attribute_id: request.attribute_id,
        seams: request.seams.clone(),
        pins: request.pins.clone(),
        atlas: None,
    };
    layout.validate(&output)?;
    let output_bytes = (canonical(&output)?.len() + canonical(&layout)?.len()) as u64;
    if output_bytes > budget.max_output_bytes {
        return Err(Error::new("budget", "UV output byte budget exceeded"));
    }
    check(&mut cancel)?;
    let report = Report {
        profile: PROFILE.into(),
        source_mesh: source,
        output_mesh: key,
        layout: layout.content_id()?,
        output_bytes,
        solver_work: work,
        packing: None,
        charts: reports,
    };
    Ok(Computed {
        mesh: output,
        layout,
        report,
    })
}
