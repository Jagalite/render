//! Deterministic bounded rectangle packing with explicit pin movement policy.
use super::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PinPacking {
    Preserve,
    TransformWithChart,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pack {
    pub atlas: Atlas,
    pub pins: PinPacking,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackingReport {
    pub transforms: Vec<ChartTransform>,
    pub placement_candidates: u64,
    pub allocated_pixels: u64,
    pub atlas_pixels: u64,
    pub surface_occupancy: f64,
    pub guard_pixels: u32,
    pub pins: PinPacking,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChartTransform {
    pub anchor_face: u64,
    pub uv_scale: [f64; 2],
    pub uv_offset: [f64; 2],
    pub preserved_pins: bool,
}
impl Atlas {
    fn validate(&self) -> Result<()> {
        if self.width == 0
            || self.height == 0
            || self.width > 4096
            || self.height > 4096
            || self.padding_pixels > 32
            || !self.pixels_per_meter.is_finite()
            || self.pixels_per_meter <= 0.
            || self.pixels_per_meter > 65536.
        {
            return Err(Error::new(
                "budget",
                "UV atlas dimensions, padding or density exceed profile",
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug)]
struct Rect {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}
impl Rect {
    fn overlaps(self, other: Self) -> bool {
        self.x0 < other.x1 && other.x0 < self.x1 && self.y0 < other.y1 && other.y0 < self.y1
    }
}
fn rect(atlas: &Atlas, r: &ChartReport, padding: f64) -> Rect {
    Rect {
        x0: r.bounds[0][0] * f64::from(atlas.width) - padding,
        y0: r.bounds[0][1] * f64::from(atlas.height) - padding,
        x1: r.bounds[1][0] * f64::from(atlas.width) + padding,
        y1: r.bounds[1][1] * f64::from(atlas.height) + padding,
    }
}
fn density(atlas: &Atlas, r: &ChartReport) -> f64 {
    (r.uv_area * f64::from(atlas.width) * f64::from(atlas.height) / r.surface_area_square_meters)
        .sqrt()
}
pub(super) fn validate(atlas: &Atlas, reports: &[ChartReport]) -> Result<()> {
    atlas.validate()?;
    let mut placed = Vec::new();
    for r in reports {
        let bounds = rect(atlas, r, f64::from(atlas.padding_pixels));
        if bounds.x0 < 0.
            || bounds.y0 < 0.
            || bounds.x1 > f64::from(atlas.width)
            || bounds.y1 > f64::from(atlas.height)
        {
            return Err(Error::new(
                "uv_atlas",
                "UV chart violates atlas bounds or padding",
            ));
        }
        if placed.iter().any(|p| bounds.overlaps(*p)) {
            return Err(Error::new(
                "uv_atlas",
                "UV chart padding rectangles overlap",
            ));
        }
        if (density(atlas, r) / atlas.pixels_per_meter - 1.).abs() > 2e-5 {
            return Err(Error::new(
                "uv_density",
                "UV chart area-average texel density differs from the authored target",
            ));
        }
        placed.push(bounds);
    }
    Ok(())
}
pub fn pack(
    mesh: &Mesh,
    layout: &Layout,
    request: &Pack,
    budget: &Budget,
    mut cancel: impl FnMut() -> bool,
) -> Result<Computed> {
    check(&mut cancel)?;
    budget.validate()?;
    if mesh
        .attributes
        .values()
        .any(|a| matches!(a.semantic.as_str(), "tangent" | "tangent_sign"))
    {
        return Err(Error::new(
            "uv_frame",
            "UV packing requires generated tangent frames; explicitly resolve authored tangent data first",
        ));
    }
    layout.validate(mesh)?;
    request.atlas.validate()?;
    let charts = charts::analyze(mesh, &layout.seams, &mut cancel)?;
    let original = values(mesh, &layout.attribute, layout.attribute_id)?;
    let mut uv = original.to_vec();
    let mut reports = Vec::new();
    for chart in &charts {
        reports.push(measure(mesh, chart, original, &mut cancel)?);
    }
    let pins = pin_indices(mesh, &layout.pins)?;
    let atlas = &request.atlas;
    let guard = f64::from(atlas.padding_pixels) + 1.;
    let mut placed = Vec::<Rect>::new();
    let mut free = Vec::new();
    let mut transforms = Vec::new();
    for (i, chart) in charts.iter().enumerate() {
        check(&mut cancel)?;
        let locked = request.pins == PinPacking::Preserve
            && pins.keys().any(|c| chart.corners.contains_key(c));
        if locked {
            let r = rect(atlas, &reports[i], guard);
            let r = Rect {
                x0: r.x0.floor(),
                y0: r.y0.floor(),
                x1: r.x1.ceil(),
                y1: r.y1.ceil(),
            };
            if r.x0 < 0.
                || r.y0 < 0.
                || r.x1 > f64::from(atlas.width)
                || r.y1 > f64::from(atlas.height)
                || placed.iter().any(|p| r.overlaps(*p))
            {
                return Err(Error::new(
                    "uv_pins",
                    "pinned chart cannot fit atlas bounds, guard or existing pinned charts",
                ));
            }
            if (density(atlas, &reports[i]) / atlas.pixels_per_meter - 1.).abs() > 2e-5 {
                return Err(Error::new(
                    "uv_pins",
                    "preserved pins determine a different texel density",
                ));
            }
            placed.push(r);
            transforms.push(ChartTransform {
                anchor_face: reports[i].anchor_face,
                uv_scale: [1.; 2],
                uv_offset: [0.; 2],
                preserved_pins: true,
            });
        } else {
            let r = &reports[i];
            let scale = atlas.pixels_per_meter * (r.surface_area_square_meters / r.uv_area).sqrt();
            let width = ((r.bounds[1][0] - r.bounds[0][0]) * scale).ceil() + 2. * guard;
            let height = ((r.bounds[1][1] - r.bounds[0][1]) * scale).ceil() + 2. * guard;
            if !scale.is_finite()
                || scale <= 0.
                || width > f64::from(atlas.width)
                || height > f64::from(atlas.height)
            {
                return Err(Error::new(
                    "uv_atlas",
                    "UV density/padding chart cannot fit atlas",
                ));
            }
            free.push((i, scale, width, height));
        }
    }
    // Largest rectangle first, then stable authored face identity. Placement is
    // bottom-left among existing rectangle-edge candidates, without rotation.
    free.sort_by(|a, b| {
        (b.2 * b.3)
            .total_cmp(&(a.2 * a.3))
            .then_with(|| reports[a.0].anchor_face.cmp(&reports[b.0].anchor_face))
    });
    let mut placements = 0u64;
    for (i, scale, width, height) in free {
        check(&mut cancel)?;
        let mut xs = vec![0.];
        let mut ys = vec![0.];
        for p in &placed {
            xs.push(p.x1);
            ys.push(p.y1);
        }
        xs.sort_by(f64::total_cmp);
        ys.sort_by(f64::total_cmp);
        xs.dedup();
        ys.dedup();
        let mut found = None;
        'search: for y in ys {
            for &x in &xs {
                check(&mut cancel)?;
                placements += 1;
                if placements > 262144 {
                    return Err(Error::new(
                        "budget",
                        "UV rectangle placement work limit exceeded",
                    ));
                }
                let r = Rect {
                    x0: x,
                    y0: y,
                    x1: x + width,
                    y1: y + height,
                };
                if r.x1 <= f64::from(atlas.width)
                    && r.y1 <= f64::from(atlas.height)
                    && !placed.iter().any(|p| r.overlaps(*p))
                {
                    found = Some(r);
                    break 'search;
                }
            }
        }
        let r = found.ok_or_else(|| {
            Error::new(
                "uv_atlas",
                "bounded rectangle placement cannot fit requested density",
            )
        })?;
        placed.push(r);
        transforms.push(ChartTransform {
            anchor_face: reports[i].anchor_face,
            uv_scale: [
                scale / f64::from(atlas.width),
                scale / f64::from(atlas.height),
            ],
            uv_offset: [
                (r.x0 + guard - reports[i].bounds[0][0] * scale) / f64::from(atlas.width),
                (r.y0 + guard - reports[i].bounds[0][1] * scale) / f64::from(atlas.height),
            ],
            preserved_pins: false,
        });
        for &c in charts[i].corners.keys() {
            uv[c] = [
                ((f64::from(original[c][0]) - reports[i].bounds[0][0]) * scale + r.x0 + guard)
                    / f64::from(atlas.width),
                ((f64::from(original[c][1]) - reports[i].bounds[0][1]) * scale + r.y0 + guard)
                    / f64::from(atlas.height),
            ]
            .map(|v| v as f32);
        }
    }
    let mut output = mesh.clone();
    output
        .attributes
        .get_mut(&layout.attribute)
        .expect("validated UV attribute")
        .values = AttributeValues::Vec2(uv.clone());
    output.validate()?;
    let mut out_layout = layout.clone();
    out_layout.mesh = output.content_id()?;
    out_layout.atlas = Some(atlas.clone());
    if request.pins == PinPacking::TransformWithChart {
        let lookup: BTreeMap<_, _> = mesh
            .corner_ids
            .iter()
            .enumerate()
            .map(|(i, id)| (*id, i))
            .collect();
        for pin in &mut out_layout.pins {
            pin.uv = uv[lookup[&pin.corner]];
        }
    }
    out_layout.validate(&output)?;
    let mut reports = Vec::new();
    for chart in &charts {
        reports.push(measure(&output, chart, &uv, &mut cancel)?);
    }
    let bytes = (canonical(&output)?.len() + canonical(&out_layout)?.len()) as u64;
    if bytes > budget.max_output_bytes {
        return Err(Error::new(
            "budget",
            "UV packing output byte budget exceeded",
        ));
    }
    check(&mut cancel)?;
    transforms.sort_by_key(|t| t.anchor_face);
    let report = Report {
        profile: "uv-rectangle-atlas-v1".into(),
        source_mesh: mesh.content_id()?,
        output_mesh: out_layout.mesh.clone(),
        layout: out_layout.content_id()?,
        output_bytes: bytes,
        solver_work: 0,
        packing: Some(PackingReport {
            transforms,
            placement_candidates: placements,
            allocated_pixels: placed
                .iter()
                .map(|r| ((r.x1 - r.x0) * (r.y1 - r.y0)) as u64)
                .sum(),
            atlas_pixels: u64::from(atlas.width) * u64::from(atlas.height),
            surface_occupancy: reports.iter().map(|r| r.uv_area).sum(),
            guard_pixels: 1,
            pins: request.pins,
        }),
        charts: reports,
    };
    Ok(Computed {
        mesh: output,
        layout: out_layout,
        report,
    })
}
