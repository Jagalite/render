//! Bounded GPU shutter accumulation. Rust owns exact-time scene evaluation;
//! ordered path dispatches accumulate f32 color in one persistent device buffer.
use super::*;
use render_core::{
    document::Snapshot,
    render::Evaluator,
    sequence::{self, Frame, FrameRequest, SequenceRequest},
};
impl Renderer {
    pub async fn render_frame(
        &mut self,
        snapshot: &Snapshot,
        request: &FrameRequest,
        cancelled: &AtomicBool,
    ) -> Result<Frame> {
        self.render_frame_observed(snapshot, request, cancelled, |_| {})
            .await
    }
    async fn render_frame_observed(
        &mut self,
        snapshot: &Snapshot,
        request: &FrameRequest,
        cancelled: &AtomicBool,
        mut submitted: impl FnMut(usize),
    ) -> Result<Frame> {
        let check = || {
            if cancelled.load(Ordering::Acquire) {
                Err(Error::new("cancelled", "GPU frame cancelled"))
            } else {
                Ok(())
            }
        };
        check()?;
        if self.is_lost() {
            return Err(Error::new(
                "device_lost",
                "recreate GPU resources from snapshot",
            ));
        }
        let times = request.validate(snapshot)?;
        let settings = snapshot
            .render_settings
            .as_ref()
            .expect("validated settings");
        let mut evaluator = Evaluator::default();
        let (center, evaluation) =
            evaluator.evaluate_at(snapshot, request.clip, request.time, || {
                cancelled.load(Ordering::Acquire)
            })?;
        // Validate supported materials/device budgets before allocating frame state.
        pack(&center, settings, 0)?;
        let mut image = if times == [request.time] {
            self.render_observed(&center, settings, 0, cancelled, || submitted(0))
                .await?
        } else {
            let size = u64::from(settings.width) * u64::from(settings.height) * 32;
            if size > self.capabilities.max_buffer_bytes
                || size > u64::from(self.capabilities.max_storage_binding_bytes)
            {
                return Err(Error::new(
                    "budget",
                    "GPU shutter buffer exceeds device limits",
                ));
            }
            self.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
            self.device.push_error_scope(wgpu::ErrorFilter::Validation);
            let output = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("GPU shutter color and nominal passes"),
                size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            let validation = self.device.pop_error_scope().await;
            let allocation = self.device.pop_error_scope().await;
            if let Some(error) = validation.or(allocation) {
                return Err(fail("gpu_execution", error));
            }
            self.render_pixels(
                &center,
                settings,
                0,
                cancelled,
                || submitted(0),
                Some(FrameTarget {
                    buffer: &output,
                    weight: 0.,
                    accumulate: false,
                    preserve_passes: false,
                    readback: false,
                }),
            )
            .await?;
            let mut result = None;
            for (index, &time) in times.iter().enumerate() {
                check()?;
                let (scene, _) =
                    Evaluator::default().evaluate_at(snapshot, request.clip, time, || {
                        cancelled.load(Ordering::Acquire)
                    })?;
                let last = index + 1 == times.len();
                let values = self
                    .render_pixels(
                        &scene,
                        settings,
                        0,
                        cancelled,
                        || submitted(index + 1),
                        Some(FrameTarget {
                            buffer: &output,
                            weight: 1. / times.len() as f32,
                            accumulate: true,
                            preserve_passes: true,
                            readback: last,
                        }),
                    )
                    .await?;
                if last {
                    result = Some(self.image_from_values(&center, settings, 0, values)?);
                }
            }
            result.expect("nonempty validated shutter")
        };
        check()?;
        image.receipt.approximation.push_str("; GPU f32 temporal color accumulation; one final image readback; intermediate 4-byte submission fences; nominal pass object indices decoded against nominal scene");
        let backend = format!("gpu-f32-animation-v0/{}", self.capabilities.backend);
        sequence::finish_frame(request, evaluation, times, image, &backend)
    }
    /// Sequential bounded frames; emitted frames remain valid when a later frame
    /// fails. The host owns the explicit partial/complete artifact manifest.
    pub async fn render_sequence(
        &mut self,
        snapshot: &Snapshot,
        request: &SequenceRequest,
        mut emit: impl FnMut(usize, &Frame) -> Result<()>,
        cancelled: &AtomicBool,
    ) -> Result<Vec<RenderReceipt>> {
        if cancelled.load(Ordering::Acquire) {
            return Err(Error::new("cancelled", "GPU sequence cancelled"));
        }
        request.validate(snapshot)?;
        let mut receipts = vec![];
        for (index, &time) in request.times.iter().enumerate() {
            let frame = self
                .render_frame(
                    snapshot,
                    &FrameRequest {
                        revision: request.revision.clone(),
                        clip: request.clip,
                        time,
                        shutter: request.shutter.clone(),
                    },
                    cancelled,
                )
                .await?;
            if cancelled.load(Ordering::Acquire) {
                return Err(Error::new(
                    "cancelled",
                    "GPU sequence cancelled before frame publication",
                ));
            }
            emit(index, &frame)?;
            receipts.push(frame.image.receipt);
        }
        Ok(receipts)
    }
    /// Cancel after a real temporal dispatch, then discard all unpublished color.
    pub async fn frame_cancellation_fault_probe(
        &mut self,
        snapshot: &Snapshot,
        request: &FrameRequest,
    ) -> Result<bool> {
        let cancelled = AtomicBool::new(false);
        match self
            .render_frame_observed(snapshot, request, &cancelled, |index| {
                if index == 2 {
                    cancelled.store(true, Ordering::Release);
                }
            })
            .await
        {
            Err(error) if error.code == "cancelled" => Ok(true),
            Err(error) => Err(error),
            Ok(_) => Ok(false),
        }
    }
}
