//! Authored spline/point geometry and bounded disposable polygon evaluation.
use crate::{
    Error, Id, Result, canonical, digest,
    geometry::{Attribute, AttributeValues, Domain, Mesh, Positions, Transfer},
};
use glam::{DQuat, DVec3};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Control {
    pub id: Id,
    pub position: [f64; 3],
    pub radius: f64,
    pub tilt: f64,
    /// Linear straight RGBA; missing means white. Explicit colors require a polyline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<[f32; 4]>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Basis {
    Polyline,
    CubicBezier,
    RationalBSpline {
        degree: u8,
        knots: Vec<f64>,
        weights: Vec<f64>,
    },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Curve {
    pub id: Id,
    pub basis: Basis,
    pub controls: Vec<Control>,
    pub closed: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Point {
    pub id: Id,
    pub position: [f64; 3],
    pub radius: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Shape {
    Curves { curves: Vec<Curve> },
    Points { points: Vec<Point> },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tessellation {
    pub chord_error: f64,
    pub radial_error: f64,
    pub max_samples: u32,
    pub max_vertices: u32,
    pub max_depth: u8,
}
impl Default for Tessellation {
    fn default() -> Self {
        Self {
            chord_error: 0.001,
            radial_error: 0.001,
            max_samples: 8192,
            max_vertices: 65536,
            max_depth: 18,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub shape: Shape,
    pub tessellation: Tessellation,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversion {
    pub source_digest: String,
    pub policy_digest: String,
    pub curve_ranges: BTreeMap<Id, [u32; 2]>,
    pub point_ranges: BTreeMap<Id, [u32; 2]>,
    pub samples: usize,
    pub vertices: usize,
    pub triangles: usize,
    pub derived_bytes: usize,
    pub approximation: String,
}
#[derive(Debug, Clone)]
pub struct Evaluated {
    pub mesh: Mesh,
    pub receipt: Conversion,
}
#[derive(Debug, Clone, Copy)]
pub struct Sample {
    pub position: DVec3,
    pub radius: f64,
    pub tilt: f64,
    pub color: [f32; 4],
}
fn error(message: &str) -> Error {
    Error::new("curve", message)
}
impl Tessellation {
    pub fn validate(&self) -> Result<()> {
        if ![self.chord_error, self.radial_error]
            .iter()
            .all(|n| n.is_finite() && *n > 0.)
            || !(2..=131072).contains(&self.max_samples)
            || !(6..=524288).contains(&self.max_vertices)
            || !(1..=24).contains(&self.max_depth)
        {
            return Err(Error::new("budget", "invalid curve tessellation limits"));
        }
        Ok(())
    }
}
impl Curve {
    pub fn has_colors(&self) -> bool {
        self.controls.iter().any(|c| c.color.is_some())
    }
    pub fn validate(&self) -> Result<()> {
        if !(2..=4096).contains(&self.controls.len()) {
            return Err(error("curve requires 2..4096 controls"));
        }
        let mut ids = BTreeSet::new();
        for p in &self.controls {
            if !ids.insert(p.id)
                || !p.position.iter().all(|v| v.is_finite() && v.abs() <= 1e12)
                || !p.radius.is_finite()
                || p.radius <= 0.
                || p.radius > 1e6
                || !p.tilt.is_finite()
                || p.tilt.abs() > 1e6
                || p.color.is_some_and(|rgba| {
                    rgba.iter()
                        .any(|v| !v.is_finite() || !(0.0..=1.).contains(v))
                })
            {
                return Err(error("invalid/duplicate control, radius, tilt or RGBA"));
            }
        }
        if self.has_colors() && !matches!(self.basis, Basis::Polyline) {
            return Err(Error::new(
                "unsupported_curve_color",
                "explicit control RGBA requires a polyline color-transfer profile",
            ));
        }
        match &self.basis {
            Basis::Polyline => {
                if self
                    .controls
                    .windows(2)
                    .any(|w| w[0].position == w[1].position)
                {
                    return Err(error("polyline has coincident adjacent controls"));
                }
                if self.closed
                    && (self.controls.len() < 3
                        || self.controls[0].position
                            == self.controls.last().expect("controls").position)
                {
                    return Err(error("closed polyline needs distinct closing controls"));
                }
            }
            Basis::CubicBezier => {
                if self.controls.len() < 4 || !(self.controls.len() - 1).is_multiple_of(3) {
                    return Err(error("cubic Bezier chain requires 3n+1 controls"));
                }
            }
            Basis::RationalBSpline {
                degree,
                knots,
                weights,
            } => {
                let p = usize::from(*degree);
                let n = self.controls.len();
                if !(1..=5).contains(degree)
                    || n <= p
                    || knots.len() != n + p + 1
                    || weights.len() != n
                    || knots.iter().any(|x| !x.is_finite())
                    || knots.windows(2).any(|w| w[0] > w[1])
                    || weights
                        .iter()
                        .any(|w| !w.is_finite() || !(1e-9..=1e9).contains(w))
                {
                    return Err(error(
                        "invalid rational spline degree, knot vector or positive weights",
                    ));
                }
                if knots[p] >= knots[n]
                    || knots[..=p].iter().any(|u| *u != knots[p])
                    || knots[n..].iter().any(|u| *u != knots[n])
                {
                    return Err(error(
                        "rational spline requires nonempty clamped knot domain",
                    ));
                }
                let mut run = 0;
                let mut previous = f64::NEG_INFINITY;
                for &u in &knots[p + 1..n] {
                    run = if u == previous { run + 1 } else { 1 };
                    previous = u;
                    if u == knots[p] || u == knots[n] || run > p {
                        return Err(error("interior knot multiplicity exceeds degree"));
                    }
                }
            }
        }
        if self.closed
            && !matches!(self.basis, Basis::Polyline)
            && self.controls[0].position != self.controls.last().expect("controls").position
        {
            return Err(error("closed spline endpoint positions must agree"));
        }
        Ok(())
    }
    /// Evaluate original basis at normalized parameter, without tessellation or state mutation.
    pub fn sample(&self, t: f64) -> Result<Sample> {
        self.validate()?;
        if !t.is_finite() || !(0.0..=1.).contains(&t) {
            return Err(error("curve parameter outside [0,1]"));
        }
        let h = match &self.basis {
            Basis::Polyline => {
                let segments = if self.closed {
                    self.controls.len()
                } else {
                    self.controls.len() - 1
                };
                let u = t * segments as f64;
                let i = (u.floor() as usize).min(segments - 1);
                hom(&self.controls[i], 1.).lerp(
                    hom(&self.controls[(i + 1) % self.controls.len()], 1.),
                    u - i as f64,
                )
            }
            Basis::CubicBezier => {
                let segments = (self.controls.len() - 1) / 3;
                let u = t * segments as f64;
                let i = (u.floor() as usize).min(segments - 1);
                casteljau(
                    &self.controls[i * 3..i * 3 + 4]
                        .iter()
                        .map(|p| hom(p, 1.))
                        .collect::<Vec<_>>(),
                    u - i as f64,
                )
            }
            Basis::RationalBSpline {
                degree,
                knots,
                weights,
            } => {
                let p = usize::from(*degree);
                let n = self.controls.len();
                let u = knots[p] + t * (knots[n] - knots[p]);
                let k = if t == 1. {
                    n - 1
                } else {
                    (p..n)
                        .find(|&i| knots[i] <= u && u < knots[i + 1])
                        .ok_or_else(|| error("knot span"))?
                };
                let mut d = (k - p..=k)
                    .map(|i| hom(&self.controls[i], weights[i]))
                    .collect::<Vec<_>>();
                for r in 1..=p {
                    for j in (r..=p).rev() {
                        let i = k - p + j;
                        let alpha = (u - knots[i]) / (knots[i + p - r + 1] - knots[i]);
                        d[j] = d[j - 1].lerp(d[j], alpha);
                    }
                }
                d[p]
            }
        };
        h.sample()
    }
    fn spans(&self) -> Result<Vec<Vec<H>>> {
        self.validate()?;
        match &self.basis {
            Basis::Polyline => {
                let mut spans = self
                    .controls
                    .windows(2)
                    .map(|w| vec![hom(&w[0], 1.), hom(&w[1], 1.)])
                    .collect::<Vec<_>>();
                if self.closed {
                    spans.push(vec![
                        hom(self.controls.last().expect("controls"), 1.),
                        hom(&self.controls[0], 1.),
                    ]);
                }
                Ok(spans)
            }
            Basis::CubicBezier => Ok((0..self.controls.len() - 1)
                .step_by(3)
                .map(|i| self.controls[i..i + 4].iter().map(|c| hom(c, 1.)).collect())
                .collect()),
            Basis::RationalBSpline {
                degree,
                knots,
                weights,
            } => {
                // Homogeneous knot insertion preserves the rational curve exactly. Saturating
                // internal knots to degree yields rational Bezier control hulls per span.
                let p = usize::from(*degree);
                let mut u = knots.clone();
                let mut cp = self
                    .controls
                    .iter()
                    .zip(weights)
                    .map(|(c, &w)| hom(c, w))
                    .collect::<Vec<_>>();
                let mut inner = knots[p + 1..self.controls.len()].to_vec();
                inner.dedup();
                for value in inner {
                    while u.iter().filter(|&&x| x == value).count() < p {
                        let k = (p..cp.len())
                            .rfind(|&i| u[i] <= value && value < u[i + 1])
                            .ok_or_else(|| error("knot insertion span"))?;
                        let multiplicity = u.iter().filter(|&&x| x == value).count();
                        let mut next = vec![H([0.; 10]); cp.len() + 1];
                        next[..=k - p].copy_from_slice(&cp[..=k - p]);
                        next[k - multiplicity + 1..].copy_from_slice(&cp[k - multiplicity..]);
                        for i in k - p + 1..=k - multiplicity {
                            let alpha = (value - u[i]) / (u[i + p] - u[i]);
                            next[i] = cp[i - 1].lerp(cp[i], alpha);
                        }
                        cp = next;
                        u.insert(k + 1, value);
                    }
                }
                Ok((p..cp.len())
                    .filter(|&k| u[k] < u[k + 1])
                    .map(|k| cp[k - p..=k].to_vec())
                    .collect())
            }
        }
    }
    pub fn tessellate(
        &self,
        policy: &Tessellation,
        cancel: &mut impl FnMut() -> bool,
    ) -> Result<Vec<Sample>> {
        policy.validate()?;
        let spans = self.spans()?;
        let mut out = vec![];
        for span in spans {
            if out.is_empty() {
                out.push(span[0].sample()?);
            }
            let mut stack = vec![(span, 0u8)];
            while let Some((points, depth)) = stack.pop() {
                if cancel() {
                    return Err(Error::new("cancelled", "curve subdivision cancelled"));
                }
                let samples = points
                    .iter()
                    .map(|p| p.sample())
                    .collect::<Result<Vec<_>>>()?;
                let a = samples[0];
                let b = *samples.last().expect("span");
                let direction = b.position - a.position;
                let len2 = direction.length_squared();
                let deviation = samples
                    .iter()
                    .map(|s| {
                        let t = if len2 > 1e-24 {
                            ((s.position - a.position).dot(direction) / len2).clamp(0., 1.)
                        } else {
                            0.
                        };
                        s.position.distance(a.position + t * direction)
                    })
                    .fold(0., f64::max);
                let range = |f: fn(&Sample) -> f64| {
                    samples.iter().map(f).fold(f64::NEG_INFINITY, f64::max)
                        - samples.iter().map(f).fold(f64::INFINITY, f64::min)
                };
                let radius = samples.iter().map(|s| s.radius).fold(0., f64::max);
                if deviation <= policy.chord_error
                    && range(|s| s.radius) <= policy.radial_error
                    && range(|s| s.tilt) * radius <= policy.radial_error
                {
                    if out.len() >= policy.max_samples as usize {
                        return Err(Error::new("budget", "curve sample limit exceeded"));
                    }
                    out.push(b);
                } else {
                    if depth >= policy.max_depth {
                        return Err(Error::new(
                            "convergence",
                            "curve error tolerance exceeds subdivision depth",
                        ));
                    }
                    let (left, right) = split(&points);
                    stack.push((right, depth + 1));
                    stack.push((left, depth + 1));
                }
            }
        }
        if self.closed {
            out.pop();
        }
        if out.len() < 2
            || out
                .windows(2)
                .any(|w| w[0].position.distance_squared(w[1].position) < 1e-24)
        {
            return Err(error("degenerate tessellated centerline"));
        }
        Ok(out)
    }
}
#[derive(Clone, Copy)]
struct H([f64; 10]);
fn hom(c: &Control, w: f64) -> H {
    let color = c.color.unwrap_or([1.; 4]);
    H([
        c.position[0] * w,
        c.position[1] * w,
        c.position[2] * w,
        c.radius * w,
        c.tilt * w,
        w,
        f64::from(color[0]) * w,
        f64::from(color[1]) * w,
        f64::from(color[2]) * w,
        f64::from(color[3]) * w,
    ])
}
impl H {
    fn lerp(self, b: Self, t: f64) -> Self {
        Self(std::array::from_fn(|i| self.0[i] * (1. - t) + b.0[i] * t))
    }
    fn sample(self) -> Result<Sample> {
        if self.0.iter().any(|x| !x.is_finite()) || self.0[5] <= 0. {
            return Err(error("nonfinite rational evaluation"));
        }
        Ok(Sample {
            position: DVec3::new(self.0[0], self.0[1], self.0[2]) / self.0[5],
            radius: self.0[3] / self.0[5],
            tilt: self.0[4] / self.0[5],
            color: std::array::from_fn(|i| (self.0[6 + i] / self.0[5]).clamp(0., 1.) as f32),
        })
    }
}
fn casteljau(points: &[H], t: f64) -> H {
    let mut p = points.to_vec();
    for n in (1..p.len()).rev() {
        for i in 0..n {
            p[i] = p[i].lerp(p[i + 1], t);
        }
    }
    p[0]
}
fn split(points: &[H]) -> (Vec<H>, Vec<H>) {
    let mut p = points.to_vec();
    let mut left = vec![p[0]];
    let mut right = vec![*p.last().expect("span")];
    for n in (1..p.len()).rev() {
        for i in 0..n {
            p[i] = p[i].lerp(p[i + 1], 0.5);
        }
        left.push(p[0]);
        right.push(p[n - 1]);
    }
    right.reverse();
    (left, right)
}
impl Asset {
    pub fn has_colors(&self) -> bool {
        matches!(&self.shape, Shape::Curves { curves } if curves.iter().any(Curve::has_colors))
    }
    pub fn validate(&self) -> Result<()> {
        self.tessellation.validate()?;
        let mut ids = BTreeSet::new();
        match &self.shape {
            Shape::Curves { curves } => {
                if curves.is_empty() || curves.len() > 4096 {
                    return Err(error("curve asset count outside 1..4096"));
                }
                let mut controls = 0;
                for curve in curves {
                    curve.validate()?;
                    if !ids.insert(curve.id) {
                        return Err(error("duplicate curve ID"));
                    }
                    controls += curve.controls.len();
                }
                if controls > 65536 {
                    return Err(Error::new("budget", "curve control budget exceeded"));
                }
            }
            Shape::Points { points } => {
                if points.is_empty() || points.len() > 16384 {
                    return Err(error("point count outside 1..16384"));
                }
                for p in points {
                    if !ids.insert(p.id)
                        || !p.position.iter().all(|x| x.is_finite() && x.abs() <= 1e12)
                        || !p.radius.is_finite()
                        || p.radius <= 0.
                        || p.radius > 1e6
                    {
                        return Err(error("invalid point or radius"));
                    }
                }
            }
        }
        Ok(())
    }
    pub fn content_id(&self) -> Result<String> {
        self.validate()?;
        Ok(digest(&canonical(self)?))
    }
    pub fn evaluate(&self, mut cancel: impl FnMut() -> bool) -> Result<Evaluated> {
        self.validate()?;
        let mut builder = Builder::new(self.tessellation.max_vertices as usize, self.has_colors());
        let mut curve_ranges = BTreeMap::new();
        let mut point_ranges = BTreeMap::new();
        let mut count = 0;
        match &self.shape {
            Shape::Curves { curves } => {
                for curve in curves {
                    let mut policy = self.tessellation.clone();
                    policy.max_samples = policy
                        .max_samples
                        .checked_sub(count as u32)
                        .filter(|n| *n >= 2)
                        .ok_or_else(|| Error::new("budget", "aggregate curve samples exceeded"))?;
                    let samples = curve.tessellate(&policy, &mut cancel)?;
                    count += samples.len();
                    let start = builder.positions.len();
                    builder.tube(&samples, curve.closed, &self.tessellation, &mut cancel)?;
                    curve_ranges.insert(curve.id, [start as u32, builder.positions.len() as u32]);
                }
            }
            Shape::Points { points } => {
                for point in points {
                    if cancel() {
                        return Err(Error::new("cancelled", "point evaluation cancelled"));
                    }
                    let start = builder.positions.len();
                    builder.sphere(point, &self.tessellation, &mut cancel)?;
                    point_ranges.insert(point.id, [start as u32, builder.positions.len() as u32]);
                    count += 1;
                }
            }
        }
        let mut mesh = Mesh::from_polygons(Positions::F64(builder.positions), &builder.faces, &[])?;
        let uv = builder.corner_uv;
        mesh.attributes.insert(
            "curve_uv".into(),
            Attribute {
                id: Id(1),
                domain: Domain::Corner,
                semantic: "uv".into(),
                transfer: Transfer::Linear,
                values: AttributeValues::Vec2(uv),
            },
        );
        if let Some(colors) = builder.colors {
            mesh.attributes.insert(
                "curve_color".into(),
                Attribute {
                    id: Id(200),
                    domain: Domain::Point,
                    semantic: "color_rgba".into(),
                    transfer: Transfer::Linear,
                    values: AttributeValues::Vec4(colors),
                },
            );
        }
        mesh.validate()?;
        let mut receipt=Conversion{source_digest:self.content_id()?,policy_digest:digest(&canonical(&self.tessellation)?),curve_ranges,point_ranges,samples:count,vertices:mesh.positions.len(),triangles:mesh.triangles()?.len(),derived_bytes:canonical(&mesh)?.len(),approximation:"bounded rational control-hull subdivision and radius/tilt variation; polygon tube/sphere sweep with parallel-transport frames; no analytic curve intersection or self-intersection removal".into()};
        if self.has_colors() {
            receipt.approximation.push_str("; linear straight polyline control RGBA transferred to tube rings/caps and polygon vertices; missing control colors are white");
        }
        Ok(Evaluated { mesh, receipt })
    }
}
struct Builder {
    positions: Vec<[f64; 3]>,
    faces: Vec<Vec<u32>>,
    uv: Vec<[f32; 2]>,
    corner_uv: Vec<[f32; 2]>,
    max: usize,
    colors: Option<Vec<[f32; 4]>>,
}
impl Builder {
    fn new(max: usize, colored: bool) -> Self {
        Self {
            positions: vec![],
            faces: vec![],
            uv: vec![],
            corner_uv: vec![],
            max,
            colors: colored.then(Vec::new),
        }
    }
    fn face(&mut self, indices: Vec<u32>, wrap_v: bool) {
        let mut uv = indices
            .iter()
            .map(|&i| self.uv[i as usize])
            .collect::<Vec<_>>();
        for axis in 0..if wrap_v { 2 } else { 1 } {
            let low = uv.iter().map(|p| p[axis]).fold(f32::INFINITY, f32::min);
            let high = uv.iter().map(|p| p[axis]).fold(f32::NEG_INFINITY, f32::max);
            if high - low > 0.5 {
                for p in &mut uv {
                    if p[axis] < 0.5 {
                        p[axis] += 1.;
                    }
                }
            }
        }
        self.corner_uv.extend(uv);
        self.faces.push(indices);
    }
    fn reserve(&self, n: usize) -> Result<()> {
        if self
            .positions
            .len()
            .checked_add(n)
            .is_none_or(|x| x > self.max)
        {
            Err(Error::new(
                "budget",
                "derived geometry vertex limit exceeded",
            ))
        } else {
            Ok(())
        }
    }
    fn push(&mut self, p: DVec3, uv: [f32; 2]) -> u32 {
        let i = self.positions.len() as u32;
        self.positions.push(p.to_array());
        self.uv.push(uv);
        if let Some(colors) = &mut self.colors {
            colors.push([1.; 4]);
        }
        i
    }
    fn sides(radius: f64, error: f64) -> Result<usize> {
        let angle = (1. - (error / radius).min(1.)).acos();
        let n = (std::f64::consts::PI / angle).ceil().max(4.);
        if !n.is_finite() || n > 256. {
            return Err(Error::new(
                "budget",
                "radial tolerance requires more than 256 sides",
            ));
        }
        Ok(n as usize)
    }
    fn tube(
        &mut self,
        samples: &[Sample],
        closed: bool,
        policy: &Tessellation,
        cancel: &mut impl FnMut() -> bool,
    ) -> Result<()> {
        let n = samples.len();
        let sides = Self::sides(
            samples.iter().map(|p| p.radius).fold(0., f64::max),
            policy.radial_error,
        )?;
        self.reserve(n * sides + if closed { 0 } else { 2 })?;
        let tangents = (0..n)
            .map(|i| {
                let previous = if i == 0 {
                    if closed { n - 1 } else { 0 }
                } else {
                    i - 1
                };
                let next = if i + 1 == n {
                    if closed { 0 } else { n - 1 }
                } else {
                    i + 1
                };
                let mut t = samples[next].position - samples[previous].position;
                if t.length_squared() < 1e-24 {
                    t = samples[next].position - samples[i].position;
                }
                if t.length_squared() < 1e-24 {
                    return Err(error("undefined sweep tangent"));
                }
                Ok(t.normalize())
            })
            .collect::<Result<Vec<_>>>()?;
        let helper = if tangents[0].z.abs() < 0.99 {
            DVec3::Z
        } else {
            DVec3::X
        };
        let mut axes = vec![helper.cross(tangents[0]).normalize()];
        let transport = |axis: DVec3, a: DVec3, b: DVec3| {
            if a.dot(b) < -1. + 1e-10 {
                axis
            } else {
                DQuat::from_rotation_arc(a, b) * axis
            }
        };
        for i in 1..n {
            let transported = transport(axes[i - 1], tangents[i - 1], tangents[i]);
            axes.push((transported - tangents[i] * transported.dot(tangents[i])).normalize());
        }
        let twist = if closed {
            let last = transport(axes[n - 1], tangents[n - 1], tangents[0]);
            tangents[0]
                .dot(last.cross(axes[0]))
                .atan2(last.dot(axes[0]))
        } else {
            0.
        };
        let base = self.positions.len() as u32;
        for (i, p) in samples.iter().enumerate() {
            if cancel() {
                return Err(Error::new("cancelled", "tube evaluation cancelled"));
            }
            let a =
                DQuat::from_axis_angle(tangents[i], p.tilt + twist * i as f64 / n as f64) * axes[i];
            let b = tangents[i].cross(a);
            for j in 0..sides {
                let angle = std::f64::consts::TAU * j as f64 / sides as f64;
                let vertex = self.push(
                    p.position + p.radius * (a * angle.cos() + b * angle.sin()),
                    [
                        j as f32 / sides as f32,
                        i as f32 / if closed { n as f32 } else { (n - 1) as f32 },
                    ],
                );
                if let Some(colors) = &mut self.colors {
                    colors[vertex as usize] = p.color;
                }
            }
        }
        let segments = if closed { n } else { n - 1 };
        for i in 0..segments {
            for j in 0..sides {
                let a = base + (i * sides + j) as u32;
                let b = base + (i * sides + (j + 1) % sides) as u32;
                let c = base + (((i + 1) % n) * sides + (j + 1) % sides) as u32;
                let d = base + (((i + 1) % n) * sides + j) as u32;
                self.face(vec![a, b, c], closed);
                self.face(vec![a, c, d], closed);
            }
        }
        if !closed {
            let first = self.push(samples[0].position, [0.5, 0.]);
            let last = self.push(samples[n - 1].position, [0.5, 1.]);
            if let Some(colors) = &mut self.colors {
                colors[first as usize] = samples[0].color;
                colors[last as usize] = samples[n - 1].color;
            }
            for j in 0..sides {
                self.face(
                    vec![first, base + ((j + 1) % sides) as u32, base + j as u32],
                    false,
                );
                self.face(
                    vec![
                        last,
                        base + ((n - 1) * sides + j) as u32,
                        base + ((n - 1) * sides + (j + 1) % sides) as u32,
                    ],
                    false,
                );
            }
        }
        Ok(())
    }
    fn sphere(
        &mut self,
        p: &Point,
        policy: &Tessellation,
        cancel: &mut impl FnMut() -> bool,
    ) -> Result<()> {
        let sides = Self::sides(p.radius, policy.radial_error / 2.)?;
        let rings = sides.div_ceil(2);
        self.reserve((rings - 1) * sides + 2)?;
        let center = DVec3::from_array(p.position);
        let north = self.push(center + DVec3::Z * p.radius, [0.5, 0.]);
        let base = self.positions.len() as u32;
        for i in 1..rings {
            if cancel() {
                return Err(Error::new("cancelled", "point sphere cancelled"));
            }
            let latitude = std::f64::consts::PI * i as f64 / rings as f64;
            for j in 0..sides {
                let a = std::f64::consts::TAU * j as f64 / sides as f64;
                self.push(
                    center
                        + p.radius
                            * DVec3::new(
                                latitude.sin() * a.cos(),
                                latitude.sin() * a.sin(),
                                latitude.cos(),
                            ),
                    [j as f32 / sides as f32, i as f32 / rings as f32],
                );
            }
        }
        let south = self.push(center - DVec3::Z * p.radius, [0.5, 1.]);
        for j in 0..sides {
            self.face(
                vec![north, base + j as u32, base + ((j + 1) % sides) as u32],
                false,
            );
            for i in 0..rings - 2 {
                let a = base + (i * sides + j) as u32;
                let b = base + ((i + 1) * sides + j) as u32;
                let c = base + ((i + 1) * sides + (j + 1) % sides) as u32;
                let d = base + (i * sides + (j + 1) % sides) as u32;
                self.face(vec![a, b, c], false);
                self.face(vec![a, c, d], false);
            }
            self.face(
                vec![
                    south,
                    base + ((rings - 2) * sides + (j + 1) % sides) as u32,
                    base + ((rings - 2) * sides + j) as u32,
                ],
                false,
            );
        }
        Ok(())
    }
}
