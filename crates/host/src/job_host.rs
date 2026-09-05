use render_core::{
    document::Document,
    jobs::{Budget, Event, Job, RenderInput, State},
    render::{Evaluator, Settings, render},
    storage::{native::NativeStore, *},
    *,
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};
struct Inner {
    doc: Document,
    store: NativeStore,
    error: Option<Error>,
}
pub struct JobHost {
    inner: Arc<Mutex<Inner>>,
    stop: Arc<AtomicBool>,
    wake: mpsc::Sender<()>,
    worker: Option<thread::JoinHandle<()>>,
    // Held until the worker is joined in Drop. Publication locks alone cannot
    // distinguish an active worker from an interrupted previous host.
    _lease: fs::File,
}
impl JobHost {
    pub fn open(root: &Path, initial: Document) -> Result<Self> {
        fs::create_dir_all(root).map_err(|e| Error::new("storage", e.to_string()))?;
        let lease = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(root.join("worker.lock"))
            .map_err(|e| Error::new("storage", e.to_string()))?;
        lease.try_lock().map_err(|e| {
            Error::new(
                "host_busy",
                format!("exclusive job-worker lease unavailable: {e}"),
            )
        })?;
        if root.join("journal").is_dir() {
            return Err(Error::new(
                "storage_layout",
                "legacy job-only package: open its journal directory explicitly; API and jobs now use the same project root",
            ));
        }
        let store = NativeStore::open(root)?;
        let doc = recover(&store.load()?)?.unwrap_or(initial);
        let mut inner = Inner {
            doc,
            store,
            error: None,
        };
        {
            let Inner { doc, store, .. } = &mut inner;
            durable_update(store, doc, |d| {
                d.jobs_mut().recover_interrupted();
                Ok(())
            })?;
        }
        let inner = Arc::new(Mutex::new(inner));
        let stop = Arc::new(AtomicBool::new(false));
        let (wake, rx) = mpsc::channel();
        let state = inner.clone();
        let stopping = stop.clone();
        let root = root.to_owned();
        let worker = thread::spawn(move || {
            while !stopping.load(Ordering::Acquire) {
                let job = {
                    let Ok(mut inner) = state.lock() else {
                        break;
                    };
                    if inner.error.is_some() {
                        break;
                    }
                    let Inner { doc, store, .. } = &mut *inner;
                    let has_queued = doc.jobs().jobs.values().any(|j| j.state == State::Queued);
                    if has_queued {
                        match durable_update(store, doc, |d| {
                            Ok(d.jobs_mut()
                                .start_next()
                                .and_then(|id| d.jobs().jobs.get(&id).cloned()))
                        }) {
                            Ok(j) => j,
                            Err(e) => {
                                inner.error = Some(e);
                                None
                            }
                        }
                    } else {
                        None
                    }
                };
                if let Some(job) = job {
                    let start = Instant::now();
                    let result = run(&root, &job, || {
                        stopping.load(Ordering::Acquire)
                            || start.elapsed().as_millis() > u128::from(job.budget.max_wall_ms)
                            || state.lock().map_or(true, |s| {
                                s.doc
                                    .jobs()
                                    .jobs
                                    .get(&job.id)
                                    .is_none_or(|j| j.state == State::CancelRequested)
                            })
                    });
                    let Ok(mut inner) = state.lock() else {
                        break;
                    };
                    let Inner { doc, store, .. } = &mut *inner;
                    let result = if start.elapsed().as_millis() > u128::from(job.budget.max_wall_ms)
                    {
                        Err(Error::new("budget", "job wall-time budget exceeded"))
                    } else {
                        result
                    };
                    if let Err(e) = durable_update(store, doc, |d| {
                        d.jobs_mut().finish(job.id, result)?;
                        Ok(())
                    }) {
                        inner.error = Some(e);
                    }
                } else {
                    let _ = rx.recv_timeout(Duration::from_millis(25));
                }
            }
        });
        Ok(Self {
            inner,
            stop,
            wake,
            worker: Some(worker),
            _lease: lease,
        })
    }
    pub fn submit(
        &self,
        principal: &str,
        id: Id,
        settings: Settings,
        budget: Budget,
    ) -> Result<Id> {
        settings.validate()?;
        if settings.samples > budget.max_samples || settings.max_bytes > budget.max_bytes {
            return Err(Error::new("admission", "render settings exceed job budget"));
        }
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| Error::new("host", "host lock poisoned"))?;
        if let Some(e) = &inner.error {
            return Err(e.clone());
        }
        let input = RenderInput {
            snapshot: inner.doc.snapshot().clone(),
            settings,
        };
        Self::submit_locked(&mut inner, principal, id, input, budget)?;
        let _ = self.wake.send(());
        Ok(id)
    }
    /// Trusted local host accepts a validated pinned branch; the root remains
    /// unchanged and the input is durably captured in the job record.
    pub fn submit_input(
        &self,
        principal: &str,
        id: Id,
        input: RenderInput,
        budget: Budget,
    ) -> Result<Id> {
        input.snapshot.validate()?;
        input.settings.validate()?;
        if canonical(&input)?.len() > 16 * 1024 * 1024 {
            return Err(Error::new("budget", "pinned input exceeds 16 MiB"));
        }
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| Error::new("host", "host lock poisoned"))?;
        Self::submit_locked(&mut inner, principal, id, input, budget)?;
        let _ = self.wake.send(());
        Ok(id)
    }
    fn submit_locked(
        inner: &mut Inner,
        principal: &str,
        id: Id,
        input: RenderInput,
        budget: Budget,
    ) -> Result<()> {
        if let Some(e) = &inner.error {
            return Err(e.clone());
        }
        if input.settings.samples > budget.max_samples
            || input.settings.max_bytes > budget.max_bytes
        {
            return Err(Error::new("admission", "render settings exceed job budget"));
        }
        if let Some(old) = inner.doc.jobs().jobs.get(&id) {
            if old.principal != principal
                || canonical(&old.render_input)? != canonical(&Some(&input))?
                || canonical(&old.budget)? != canonical(&budget)?
            {
                return Err(Error::new(
                    "idempotency_mismatch",
                    "job ID already used with different input or owner",
                ));
            }
            return Ok(());
        }
        if inner.doc.jobs().jobs.len() >= 256 {
            return Err(Error::new(
                "budget",
                "local host retains at most 256 jobs; archive the project to reclaim history",
            ));
        }
        let job = Job {
            id,
            principal: principal.into(),
            revision: input.snapshot.revision()?,
            state: State::Accepted,
            budget,
            artifacts: vec![],
            diagnostic: None,
            render_input: Some(input),
        };
        let Inner { doc, store, .. } = inner;
        durable_update(store, doc, |d| d.jobs_mut().submit(job))?;
        Ok(())
    }
    pub fn cancel(&self, principal: &str, id: Id) -> Result<State> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| Error::new("host", "host lock poisoned"))?;
        let Inner { doc, store, .. } = &mut *inner;
        durable_update(store, doc, |d| d.jobs_mut().cancel(principal, id))
    }
    pub fn status(&self, principal: &str, id: Id) -> Result<Job> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| Error::new("host", "host lock poisoned"))?;
        if let Some(e) = &inner.error {
            return Err(e.clone());
        }
        inner
            .doc
            .jobs()
            .jobs
            .get(&id)
            .filter(|j| j.principal == principal)
            .cloned()
            .ok_or_else(|| Error::new("not_found", "job not owned by caller"))
    }
    pub fn events(&self, principal: &str, cursor: Option<u64>) -> Result<Vec<Event>> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| Error::new("host", "host lock poisoned"))?;
        Ok(inner
            .doc
            .jobs()
            .events_after(cursor)?
            .into_iter()
            .filter(|event| {
                inner
                    .doc
                    .jobs()
                    .jobs
                    .get(&event.job)
                    .is_some_and(|j| j.principal == principal)
            })
            .collect())
    }
}
impl Drop for JobHost {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let _ = self.wake.send(());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn run(root: &Path, job: &Job, cancel: impl FnMut() -> bool) -> Result<Vec<String>> {
    let input = job
        .render_input
        .as_ref()
        .ok_or_else(|| Error::new("job", "missing pinned render input"))?;
    if input.snapshot.revision()? != job.revision {
        return Err(Error::new("integrity", "job input revision mismatch"));
    }
    let scene = Evaluator::default().evaluate(&input.snapshot)?;
    let image = render(&scene, &input.settings, cancel)?;
    let ppm = image.ppm();
    let receipt = canonical(&image.receipt)?;
    let passes = canonical(
        &serde_json::json!({"depth_meters":image.depth,"normals_world":image.normals,"object_ids":image.objects,"linear_rgb":image.linear}),
    )?;
    if (ppm.len() + receipt.len() + passes.len()) as u64 > job.budget.max_output_bytes {
        return Err(Error::new("budget", "artifact exceeds job output budget"));
    }
    let folder = root.join("artifacts");
    fs::create_dir_all(&folder).map_err(|e| Error::new("io", e.to_string()))?;
    let mut artifacts = vec![];
    for (extension, bytes) in [
        ("ppm", ppm),
        ("receipt.json", receipt),
        ("passes.json", passes),
    ] {
        let path = folder.join(format!("{:032x}.{extension}", job.id.0));
        let staging = PathBuf::from(format!("{}.pending", path.display()));
        fs::write(&staging, bytes).map_err(|e| Error::new("io", e.to_string()))?;
        let file = fs::File::open(&staging).map_err(|e| Error::new("io", e.to_string()))?;
        file.sync_all()
            .map_err(|e| Error::new("io", e.to_string()))?;
        fs::rename(staging, &path).map_err(|e| Error::new("io", e.to_string()))?;
        artifacts.push(path.to_string_lossy().into_owned());
    }
    fs::File::open(folder)
        .and_then(|f| f.sync_all())
        .map_err(|e| Error::new("io", e.to_string()))?;
    Ok(artifacts)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pinned_variant_jobs_reject_changed_retries_and_recover_terminal_events() {
        let root = std::env::temp_dir().join(format!("render-m05-jobs-{}", std::process::id()));
        let initial = fixtures::demo().unwrap();
        let mut variant = initial.clone();
        let edit = fixtures::request(
            &variant,
            "job:variant:000001",
            vec![document::Command::Rename {
                entity: Id(4),
                name: "pinned branch".into(),
            }],
        )
        .unwrap();
        variant.execute(&fixtures::principal(), &edit).unwrap();
        let mut settings = fixtures::settings();
        settings.width = 8;
        settings.height = 8;
        settings.samples = 1;
        let input = RenderInput {
            snapshot: variant.snapshot().clone(),
            settings: settings.clone(),
        };
        let budget = Budget {
            max_wall_ms: 10000,
            max_bytes: settings.max_bytes,
            max_samples: 1,
            max_output_bytes: 1,
        };
        let host = JobHost::open(&root, initial.clone()).unwrap();
        let mut excess = input.clone();
        excess.settings.samples = 2;
        assert_eq!(
            host.submit_input("p", Id(70), excess, budget.clone())
                .unwrap_err()
                .code,
            "admission"
        );
        assert!(host.status("p", Id(70)).is_err());
        host.submit_input("p", Id(71), input.clone(), budget.clone())
            .unwrap();
        assert_eq!(
            host.submit_input("other", Id(71), input.clone(), budget.clone())
                .unwrap_err()
                .code,
            "idempotency_mismatch"
        );
        let mut changed = input.clone();
        changed.settings.seed += 1;
        assert_eq!(
            host.submit_input("p", Id(71), changed, budget.clone())
                .unwrap_err()
                .code,
            "idempotency_mismatch"
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if host.status("p", Id(71)).unwrap().state.terminal() {
                break;
            }
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(5));
        }
        let job = host.status("p", Id(71)).unwrap();
        assert_eq!(job.state, State::Failed);
        assert!(job.diagnostic.unwrap().contains("budget"));
        assert!(job.artifacts.is_empty());
        assert_eq!(job.revision, variant.snapshot().revision().unwrap());
        let events = host.events("p", None).unwrap();
        let cursor = events.last().unwrap().sequence;
        host.submit_input("p", Id(71), input.clone(), budget.clone())
            .unwrap();
        assert!(host.events("p", Some(cursor)).unwrap().is_empty());
        drop(host);
        let saved = recover(&NativeStore::open(&root).unwrap().load().unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(
            saved.snapshot().revision().unwrap(),
            initial.snapshot().revision().unwrap()
        );
        let reopened = JobHost::open(&root, initial).unwrap();
        reopened.submit_input("p", Id(71), input, budget).unwrap();
        assert_eq!(reopened.status("p", Id(71)).unwrap().state, State::Failed);
        assert!(reopened.events("p", Some(cursor)).unwrap().is_empty());
        drop(reopened);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn jobs_pin_the_document_written_through_the_project_api() {
        let root = std::env::temp_dir().join(format!("render-job-api-{}", std::process::id()));
        let mut authored = fixtures::demo().unwrap();
        let request = fixtures::request(
            &authored,
            "api:before:job:001",
            vec![document::Command::Rename {
                entity: Id(4),
                name: "API-authored instance".into(),
            }],
        )
        .unwrap();
        let mut store = NativeStore::open(&root).unwrap();
        let accepted =
            durable_execute(&mut store, &mut authored, &fixtures::principal(), &request).unwrap();
        let host = JobHost::open(&root, fixtures::demo().unwrap()).unwrap();
        let mut settings = fixtures::settings();
        settings.width = 4;
        settings.height = 4;
        host.submit(
            "p",
            Id(12),
            settings.clone(),
            Budget {
                max_wall_ms: 10000,
                max_bytes: settings.max_bytes,
                max_samples: 65536,
                max_output_bytes: 1000000,
            },
        )
        .unwrap();
        let job = host.status("p", Id(12)).unwrap();
        assert_eq!(job.revision, accepted.revision);
        assert_eq!(
            job.render_input
                .unwrap()
                .snapshot
                .entities
                .get(Id(4))
                .unwrap()
                .name,
            "API-authored instance"
        );
        drop(host);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn second_host_cannot_recover_an_active_worker() {
        let root = std::env::temp_dir().join(format!("render-job-lease-{}", std::process::id()));
        let initial = fixtures::demo().unwrap();
        let first = JobHost::open(&root, initial.clone()).unwrap();
        assert!(matches!(JobHost::open(&root, initial.clone()), Err(e) if e.code == "host_busy"));
        drop(first);
        let reopened = JobHost::open(&root, initial).unwrap();
        drop(reopened);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn durable_jobs_survive_reopen_and_cancel() {
        let root = std::env::temp_dir().join(format!("render-job-host-{}", std::process::id()));
        let initial = fixtures::demo().unwrap();
        let host = JobHost::open(&root, initial.clone()).unwrap();
        let mut settings = fixtures::settings();
        settings.width = 12;
        settings.height = 8;
        settings.samples = 2;
        let budget = Budget {
            max_wall_ms: 10000,
            max_bytes: settings.max_bytes,
            max_samples: 65536,
            max_output_bytes: 1000000,
        };
        host.submit("p", Id(10), settings.clone(), budget.clone())
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let job = host.status("p", Id(10)).unwrap();
            if job.state.terminal() {
                assert_eq!(job.state, State::Succeeded);
                assert_eq!(job.artifacts.len(), 3);
                break;
            }
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(5));
        }
        let cursor = host.events("p", None).unwrap().last().unwrap().sequence;
        settings.samples = 65536;
        host.submit("p", Id(11), settings, budget).unwrap();
        host.cancel("p", Id(11)).unwrap();
        drop(host);
        let restored = JobHost::open(&root, initial).unwrap();
        assert_eq!(
            restored.status("p", Id(10)).unwrap().state,
            State::Succeeded
        );
        assert_eq!(
            restored.status("p", Id(11)).unwrap().state,
            State::Cancelled
        );
        assert!(
            restored
                .events("p", Some(cursor))
                .unwrap()
                .iter()
                .all(|e| e.sequence > cursor)
        );
        drop(restored);
        fs::remove_dir_all(root).unwrap();
    }
}
