use super::{Asset, Coordinate, TILE_SIDE, Tile, check_cancel, mul16};
use crate::{Error, Id, Result, Time, canonical};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Mode {
    /// Straight linear input, quantized once to UNORM16 before premultiplication.
    Paint {
        color: [f64; 4],
    },
    Erase {
        opacity: f64,
    },
    Mask {
        value: f64,
        opacity: f64,
    },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Brush {
    pub radius: f64,
    pub hardness: f64,
    pub spacing: f64,
    pub mode: Mode,
}
fn unit(x: f64) -> bool {
    x.is_finite() && (0.0..=1.0).contains(&x)
}
pub(super) fn quantize(x: f64) -> u16 {
    (x * 65535.0 + 0.5).floor() as u16
}
impl Brush {
    pub fn validate(&self) -> Result<()> {
        let mode = match self.mode {
            Mode::Paint { color } => color.into_iter().all(unit),
            Mode::Erase { opacity } => unit(opacity),
            Mode::Mask { value, opacity } => unit(value) && unit(opacity),
        };
        if !mode
            || !self.radius.is_finite()
            || !(0.25..=128.).contains(&self.radius)
            || !unit(self.hardness)
            || !self.spacing.is_finite()
            || !(0.05..=2.).contains(&self.spacing)
        {
            return Err(Error::new(
                "paint_brush",
                "brush parameters outside paint-tiles-v1 profile",
            ));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sample {
    pub pixel: [f64; 2],
    pub pressure: f64,
    #[serde(with = "sample_time")]
    pub time: Time,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stroke {
    pub id: Id,
    pub layer: Id,
    pub brush: Brush,
    pub samples: Vec<Sample>,
    pub closed: bool,
}
impl Stroke {
    pub fn validate(&self) -> Result<()> {
        self.brush.validate()?;
        if self.samples.is_empty() || self.samples.len() > 256 {
            return Err(Error::new("budget", "stroke requires 1..256 samples"));
        }
        let mut previous: Option<Time> = None;
        for sample in &self.samples {
            if !sample
                .pixel
                .into_iter()
                .all(|v| v.is_finite() && v.abs() <= 4096.)
                || !unit(sample.pressure)
                || sample.time.numerator < 0
                || Time::new(sample.time.numerator, sample.time.denominator)? != sample.time
            {
                return Err(Error::new(
                    "paint_sample",
                    "invalid coordinate, pressure or canonical nonnegative time",
                ));
            }
            if let Some(p) = previous
                && i128::from(p.numerator) * i128::from(sample.time.denominator)
                    > i128::from(sample.time.numerator) * i128::from(p.denominator)
            {
                return Err(Error::new("paint_sample", "stroke time must be monotone"));
            }
            previous = Some(sample.time);
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrokeInput {
    pub id: Id,
    pub layer: Id,
    pub brush: Brush,
    pub samples: Vec<Sample>,
    pub finish: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Budget {
    pub max_dabs: u32,
    pub max_pixel_work: u64,
    pub max_touched_tiles: u32,
    pub max_output_bytes: u64,
}
impl Default for Budget {
    fn default() -> Self {
        Self {
            max_dabs: 4096,
            max_pixel_work: 16_777_216,
            max_touched_tiles: 64,
            max_output_bytes: 4 * 1024 * 1024,
        }
    }
}
impl Budget {
    pub fn validate(&self) -> Result<()> {
        if self.max_dabs == 0
            || self.max_dabs > 4096
            || self.max_pixel_work == 0
            || self.max_pixel_work > 16_777_216
            || self.max_touched_tiles == 0
            || self.max_touched_tiles > 64
            || self.max_output_bytes == 0
            || self.max_output_bytes > 4 * 1024 * 1024
        {
            return Err(Error::new(
                "budget",
                "paint work or output budget outside profile",
            ));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrokeReport {
    pub source: String,
    pub output: String,
    pub emitted_dabs: u32,
    pub pixel_work: u64,
    pub touched_tiles: Vec<Coordinate>,
    pub changed_tiles: Vec<Coordinate>,
    pub copied_tile_bytes: u64,
    pub output_bytes: u64,
}
pub struct Painted {
    pub asset: Asset,
    pub tiles: BTreeMap<String, Arc<Tile>>,
    pub report: StrokeReport,
}
#[derive(Clone, Copy)]
struct Dab {
    pixel: [f64; 2],
    pressure: f64,
}
// Re-derive the bounded prefix from authored samples instead of trusting serialized
// floating accumulator state. Only newly emitted dabs are applied to existing tiles.
fn dabs(stroke: &Stroke, cancel: &mut impl FnMut() -> bool) -> Result<Vec<Dab>> {
    stroke.validate()?;
    let first = &stroke.samples[0];
    let mut out = vec![Dab {
        pixel: first.pixel,
        pressure: first.pressure,
    }];
    let spacing = stroke.brush.radius * stroke.brush.spacing;
    let mut remaining = spacing;
    for pair in stroke.samples.windows(2) {
        check_cancel(cancel)?;
        let a = &pair[0];
        let b = &pair[1];
        let delta = [b.pixel[0] - a.pixel[0], b.pixel[1] - a.pixel[1]];
        let length = libm::sqrt(delta[0] * delta[0] + delta[1] * delta[1]);
        if length == 0. {
            continue;
        }
        let mut distance = remaining;
        // Snap roundoff at a spacing/segment endpoint boundary. Without this,
        // a + (b - a) can differ from b and finish would apply a second dab.
        let tolerance = 32. * f64::EPSILON * length.max(1.);
        while distance <= length + tolerance {
            if out.len() >= 4096 {
                return Err(Error::new("budget", "retained stroke exceeds 4096 dabs"));
            }
            let endpoint = (distance - length).abs() <= tolerance;
            let t = distance / length;
            out.push(Dab {
                pixel: if endpoint {
                    b.pixel
                } else {
                    [a.pixel[0] + delta[0] * t, a.pixel[1] + delta[1] * t]
                },
                pressure: if endpoint {
                    b.pressure
                } else {
                    a.pressure + (b.pressure - a.pressure) * t
                },
            });
            distance = if endpoint {
                length + spacing
            } else {
                distance + spacing
            };
        }
        remaining = distance - length;
    }
    if stroke.closed {
        let last = stroke.samples.last().expect("validated nonempty");
        if out.last().expect("initial dab").pixel != last.pixel {
            if out.len() >= 4096 {
                return Err(Error::new("budget", "stroke endpoint exceeds dab budget"));
            }
            out.push(Dab {
                pixel: last.pixel,
                pressure: last.pressure,
            });
        }
    }
    Ok(out)
}
fn coverage(brush: &Brush, dab: Dab, x: u32, y: u32) -> u16 {
    let mut total = 0.;
    for dx in [0.25, 0.75] {
        for dy in [0.25, 0.75] {
            let a = f64::from(x) + dx - dab.pixel[0];
            let b = f64::from(y) + dy - dab.pixel[1];
            let r = libm::sqrt(a * a + b * b) / brush.radius;
            total += if r >= 1. {
                0.
            } else if r <= brush.hardness {
                1.
            } else {
                (1. - r) / (1. - brush.hardness)
            };
        }
    }
    quantize(total * 0.25 * dab.pressure)
}
fn apply(tile: &mut Tile, index: usize, mode: &Mode, coverage: u16) {
    match (tile, mode) {
        (Tile::Color { rgba_le, .. }, Mode::Paint { color }) => {
            let alpha = mul16(quantize(color[3]), coverage);
            let source = [
                mul16(quantize(color[0]), alpha),
                mul16(quantize(color[1]), alpha),
                mul16(quantize(color[2]), alpha),
                alpha,
            ];
            for (dst, src) in rgba_le[index].iter_mut().zip(source) {
                *dst = src + mul16(*dst, 65535 - alpha);
            }
        }
        (Tile::Color { rgba_le, .. }, Mode::Erase { opacity }) => {
            let factor = 65535 - mul16(quantize(*opacity), coverage);
            for dst in &mut rgba_le[index] {
                *dst = mul16(*dst, factor);
            }
        }
        (Tile::Mask { coverage_le, .. }, Mode::Mask { value, opacity }) => {
            let alpha = mul16(quantize(*opacity), coverage);
            coverage_le[index] =
                mul16(quantize(*value), alpha) + mul16(coverage_le[index], 65535 - alpha);
        }
        _ => unreachable!("validated tile channel"),
    }
}
pub fn paint(
    source: &Asset,
    tiles: &BTreeMap<String, Arc<Tile>>,
    input: &StrokeInput,
    budget: &Budget,
    mut cancelled: impl FnMut() -> bool,
) -> Result<Painted> {
    check_cancel(&mut cancelled)?;
    budget.validate()?;
    source.validate_tiles(tiles)?;
    let layer_index = source
        .layers
        .iter()
        .position(|l| l.id == input.layer)
        .ok_or_else(|| Error::new("reference", "paint layer missing"))?;
    let mut asset = source.clone();
    let existing = asset.strokes.iter().position(|s| s.id == input.id);
    let (mut stroke, skip) = if let Some(index) = existing {
        let old = &asset.strokes[index];
        if old.closed || old.layer != input.layer || old.brush != input.brush {
            return Err(Error::new(
                "paint_stroke",
                "continuation requires open stroke with same layer and brush",
            ));
        }
        (old.clone(), dabs(old, &mut cancelled)?.len())
    } else {
        if asset.strokes.len() >= 16 {
            return Err(Error::new(
                "budget",
                "canvas retains at most 16 stroke recipes",
            ));
        }
        (
            Stroke {
                id: input.id,
                layer: input.layer,
                brush: input.brush.clone(),
                samples: vec![],
                closed: false,
            },
            0,
        )
    };
    if input.samples.is_empty() && (!input.finish || existing.is_none()) {
        return Err(Error::new("paint_sample", "empty stroke checkpoint"));
    }
    if stroke.samples.len() + input.samples.len() > 256 {
        return Err(Error::new("budget", "retained stroke exceeds 256 samples"));
    }
    stroke.samples.extend(input.samples.clone());
    stroke.closed = input.finish;
    let all_dabs = dabs(&stroke, &mut cancelled)?;
    let count = all_dabs.len() - skip;
    if count > budget.max_dabs as usize {
        return Err(Error::new("budget", "paint dab budget exceeded"));
    }
    let is_mask = matches!(input.brush.mode, Mode::Mask { .. });
    let layer = &mut asset.layers[layer_index];
    let refs = if is_mask {
        &mut layer.mask.0
    } else {
        &mut layer.color.0
    };
    let mut touched: BTreeMap<Coordinate, Tile> = BTreeMap::new();
    let mut work = 0u64;
    for dab in all_dabs.into_iter().skip(skip) {
        check_cancel(&mut cancelled)?;
        let radius = input.brush.radius;
        let left = (dab.pixel[0] - radius)
            .floor()
            .clamp(0., f64::from(source.width)) as u32;
        let right = (dab.pixel[0] + radius)
            .ceil()
            .clamp(0., f64::from(source.width)) as u32;
        let top = (dab.pixel[1] - radius)
            .floor()
            .clamp(0., f64::from(source.height)) as u32;
        let bottom = (dab.pixel[1] + radius)
            .ceil()
            .clamp(0., f64::from(source.height)) as u32;
        for y in top..bottom {
            check_cancel(&mut cancelled)?;
            for x in left..right {
                work += 1;
                if work > budget.max_pixel_work {
                    return Err(Error::new("budget", "paint pixel work budget exceeded"));
                }
                let coverage = coverage(&input.brush, dab, x, y);
                if coverage == 0 {
                    continue;
                }
                let c = Coordinate {
                    x: x / TILE_SIDE,
                    y: y / TILE_SIDE,
                };
                if !touched.contains_key(&c) {
                    if touched.len() >= budget.max_touched_tiles as usize {
                        return Err(Error::new("budget", "paint touched tile budget exceeded"));
                    }
                    let tile = refs
                        .get(&c)
                        .map(|hash| tiles[hash].as_ref().clone())
                        .unwrap_or_else(|| {
                            if is_mask {
                                Tile::full_mask()
                            } else {
                                Tile::transparent()
                            }
                        });
                    touched.insert(c, tile);
                }
                let index = ((y % TILE_SIDE) * TILE_SIDE + x % TILE_SIDE) as usize;
                apply(
                    touched.get_mut(&c).expect("inserted tile"),
                    index,
                    &input.brush.mode,
                    coverage,
                );
            }
        }
    }
    let mut output = BTreeMap::new();
    let mut report = StrokeReport {
        source: source.content_id()?,
        output: String::new(),
        emitted_dabs: count as u32,
        pixel_work: work,
        touched_tiles: touched.keys().copied().collect(),
        changed_tiles: vec![],
        copied_tile_bytes: touched.values().map(Tile::decoded_bytes).sum(),
        output_bytes: 0,
    };
    for (c, tile) in touched {
        check_cancel(&mut cancelled)?;
        if tile.is_default() {
            if refs.remove(&c).is_some() {
                report.changed_tiles.push(c);
            }
        } else {
            let hash = tile.content_id()?;
            if refs.get(&c) != Some(&hash) {
                refs.insert(c, hash.clone());
                report.changed_tiles.push(c);
                // Include changed payloads even if another retained asset already
                // owns them: retry command bytes must depend only on source/input.
                output.insert(hash, Arc::new(tile));
            }
        }
    }
    if let Some(index) = existing {
        asset.strokes[index] = stroke;
    } else {
        asset.strokes.push(stroke);
    }
    asset.validate()?;
    report.output = asset.content_id()?;
    report.output_bytes = canonical(&asset)?.len() as u64;
    for tile in output.values() {
        report.output_bytes += canonical(tile)?.len() as u64;
    }
    if report.output_bytes > budget.max_output_bytes {
        return Err(Error::new("budget", "paint delta byte budget exceeded"));
    }
    check_cancel(&mut cancelled)?;
    Ok(Painted {
        asset,
        tiles: output,
        report,
    })
}

// The shared Time type predates strict operation schemas; this public paint field
// rejects unknown wire fields without changing unrelated legacy time decoding.
mod sample_time {
    use super::*;
    pub fn serialize<S: serde::Serializer>(v: &Time, s: S) -> std::result::Result<S::Ok, S::Error> {
        v.serialize(s)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        d: D,
    ) -> std::result::Result<Time, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            numerator: i64,
            denominator: u64,
        }
        let v = Wire::deserialize(d)?;
        Ok(Time {
            numerator: v.numerator,
            denominator: v.denominator,
        })
    }
}
