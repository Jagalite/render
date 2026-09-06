//! Pinned, exact-time sequence rendering with bounded shutter quadrature.
use crate::{Error, Id, Result, Time, animation, canonical, digest, document::Snapshot, render::*};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shutter {
    pub open: Time,
    pub close: Time,
    pub samples: u32,
}
impl Shutter {
    pub fn instant() -> Self {
        Self {
            open: Time {
                numerator: 0,
                denominator: 1,
            },
            close: Time {
                numerator: 0,
                denominator: 1,
            },
            samples: 1,
        }
    }
    pub fn times(&self, time: Time) -> Result<Vec<Time>> {
        if self.samples == 0 || self.samples > 32 {
            return Err(Error::new(
                "budget",
                "shutter supports 1..32 temporal samples",
            ));
        }
        let duration = self.close.subtract(self.open)?;
        if duration.numerator < 0 || (duration.numerator == 0 && self.samples != 1) {
            return Err(Error::new(
                "shutter",
                "invalid interval or redundant zero-width samples",
            ));
        }
        (0..self.samples)
            .map(|i| {
                time.add_time(self.open)?
                    .add_time(duration.multiply(Time::new(
                        i64::from(2 * i + 1),
                        u64::from(2 * self.samples),
                    )?)?)
            })
            .collect()
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrameRequest {
    pub revision: String,
    pub clip: Id,
    pub time: Time,
    pub shutter: Shutter,
}
#[derive(Clone, Debug)]
pub struct Frame {
    pub image: Image,
    pub evaluation: animation::Receipt,
    pub temporal_times: Vec<Time>,
}
impl Evaluator {
    pub fn evaluate_at(
        &mut self,
        snapshot: &Snapshot,
        clip: Id,
        time: Time,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<(Scene, animation::Receipt)> {
        let evaluated = animation::evaluate(snapshot, clip, time, &mut cancelled)?;
        let mut scene = self.evaluate_with_cancel(&evaluated.snapshot, &mut cancelled)?;
        scene.revision = evaluated.receipt.evaluation_digest.clone();
        Ok((scene, evaluated.receipt))
    }
}
pub fn render_frame(
    snapshot: &Snapshot,
    request: &FrameRequest,
    mut cancelled: impl FnMut() -> bool,
) -> Result<Frame> {
    if cancelled() {
        return Err(Error::new("cancelled", "frame render cancelled"));
    }
    if snapshot.revision()? != request.revision {
        return Err(Error::new(
            "stale_revision",
            "frame request does not match authored revision",
        ));
    }
    let times = request.shutter.times(request.time)?;
    let settings = snapshot.render_settings.as_ref().ok_or_else(|| {
        Error::new(
            "render_settings",
            "sequence requires authored render settings",
        )
    })?;
    // Per-frame output is bounded; no full sequence framebuffer accumulation.
    if settings.max_bytes < u64::from(settings.width) * u64::from(settings.height) * 128 {
        return Err(Error::new(
            "budget",
            "shutter rendering requires 128 bytes per pixel working storage",
        ));
    }
    let mut evaluator = Evaluator::default();
    let (center, evaluation) =
        evaluator.evaluate_at(snapshot, request.clip, request.time, &mut cancelled)?;
    let mut image = render(&center, settings, &mut cancelled)?;
    if times != [request.time] {
        let mut sums = vec![[0f64; 3]; image.linear.len()];
        for &at in &times {
            if cancelled() {
                return Err(Error::new("cancelled", "shutter render cancelled"));
            }
            let (scene, _) = evaluator.evaluate_at(snapshot, request.clip, at, &mut cancelled)?;
            let frame = render(&scene, settings, &mut cancelled)?;
            for (out, pixel) in sums.iter_mut().zip(frame.linear) {
                for c in 0..3 {
                    out[c] += f64::from(pixel[c]);
                }
            }
        }
        image.linear = sums
            .into_iter()
            .map(|p| p.map(|x| (x / times.len() as f64) as f32))
            .collect();
    }
    image.receipt.revision = digest(&canonical(&(request, "midpoint-shutter-v0"))?);
    image.receipt.backend = "cpu-f64-animation-v0".into();
    image.receipt.approximation.push_str(&format!("; {}; {} exact-time midpoint shutter samples; rigid and LBS/morph geometry evaluated independently at each time; depth/normal/object passes at nominal frame time",evaluation.approximation,times.len()));
    image.receipt.output_digest = digest(
        &image
            .linear
            .iter()
            .flatten()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>(),
    );
    Ok(Frame {
        image,
        evaluation,
        temporal_times: times,
    })
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SequenceRequest {
    pub revision: String,
    pub clip: Id,
    pub times: Vec<Time>,
    pub shutter: Shutter,
}
/// Emits only complete frames; an error leaves preceding emitted artifacts as an
/// explicitly partial sequence. Hosts must not publish a complete manifest early.
pub fn render_sequence(
    snapshot: &Snapshot,
    request: &SequenceRequest,
    mut emit: impl FnMut(usize, &Frame) -> Result<()>,
    mut cancelled: impl FnMut() -> bool,
) -> Result<Vec<RenderReceipt>> {
    if request.times.is_empty() || request.times.len() > 1024 {
        return Err(Error::new("budget", "sequence requires 1..1024 frames"));
    }
    if snapshot.revision()? != request.revision {
        return Err(Error::new("stale_revision", "sequence revision changed"));
    }
    for pair in request.times.windows(2) {
        if pair[0].compare(pair[1])? != std::cmp::Ordering::Less {
            return Err(Error::new(
                "time",
                "sequence output times must strictly increase",
            ));
        }
    }
    let mut receipts = Vec::new();
    for (i, &time) in request.times.iter().enumerate() {
        let frame = render_frame(
            snapshot,
            &FrameRequest {
                revision: request.revision.clone(),
                clip: request.clip,
                time,
                shutter: request.shutter.clone(),
            },
            &mut cancelled,
        )?;
        emit(i, &frame)?;
        receipts.push(frame.image.receipt);
    }
    Ok(receipts)
}
