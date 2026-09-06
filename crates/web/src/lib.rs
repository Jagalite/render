//! Rust-owned browser application and storage adapter.
#[cfg(target_arch = "wasm32")]
mod browser {
    use render_core::{document::*, render::*, storage::Envelope, *};
    use std::sync::atomic::AtomicBool;
    use wasm_bindgen::{Clamped, JsCast, prelude::*};
    use wasm_bindgen_futures::{JsFuture, future_to_promise, spawn_local};
    use web_sys::{
        FileSystemDirectoryHandle, FileSystemFileHandle, FileSystemGetFileOptions,
        FileSystemWritableFileStream,
    };
    fn js(e: impl std::fmt::Display) -> JsValue {
        JsValue::from_str(&e.to_string())
    }
    /// Local browser agent client. GPU completion rechecks the pinned branch;
    /// cancellation is acknowledged separately from device submission completion.
    #[wasm_bindgen]
    pub struct BrowserAgent {
        session: std::rc::Rc<std::cell::RefCell<agent::Session>>,
        cancelled: std::sync::Arc<AtomicBool>,
        busy: std::rc::Rc<std::cell::Cell<bool>>,
    }
    fn agent_principal() -> Principal {
        Principal {
            id: "local-browser-agent".into(),
            can_write: true,
        }
    }
    fn agent_name(name: &str) -> std::result::Result<(), JsValue> {
        if name.is_empty()
            || name.len() > 64
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err(js(
                "project name must be 1..64 ASCII letters, digits, hyphens or underscores",
            ));
        }
        Ok(())
    }
    struct PreviewGuard(std::rc::Rc<std::cell::Cell<bool>>);
    impl Drop for PreviewGuard {
        fn drop(&mut self) {
            self.0.set(false);
        }
    }
    fn frame_json(frame: &sequence::Frame) -> Result<String> {
        let image = &frame.image;
        serde_json::to_string(&serde_json::json!({"receipt":image.receipt,"evaluation":frame.evaluation,"temporal_times":frame.temporal_times,"inspection":{"visible_pixels":image.objects.iter().filter(|id|id.is_some()).count()},"passes":{"linear_rgb":image.linear,"depth_meters":image.depth,"normals_world":image.normals,"object_ids":image.objects}}))
            .map_err(|e| Error::new("serialization", e.to_string()))
    }
    #[wasm_bindgen]
    impl BrowserAgent {
        #[wasm_bindgen(constructor)]
        pub fn new(id: &str) -> std::result::Result<BrowserAgent, JsValue> {
            let document = BrowserDocument::new(id)?.document;
            Ok(Self::from_document(document))
        }
        pub fn import_json(json: &str) -> std::result::Result<BrowserAgent, JsValue> {
            if json.len() > 16 * 1024 * 1024 {
                return Err(js("agent export exceeds 16 MiB"));
            }
            Ok(Self::from_document(
                BrowserDocument::import_json(json)?.document,
            ))
        }
        pub fn dispatch(&self, json: &str) -> std::result::Result<String, JsValue> {
            let bytes = self
                .session
                .borrow_mut()
                .dispatch_wire(&agent_principal(), json.as_bytes())
                .map_err(js)?;
            String::from_utf8(bytes).map_err(js)
        }
        pub fn export_json(&self) -> std::result::Result<String, JsValue> {
            String::from_utf8(canonical(self.session.borrow().document()).map_err(js)?).map_err(js)
        }
        pub fn cancel_preview(&self) -> bool {
            self.cancelled
                .store(true, std::sync::atomic::Ordering::Release);
            self.busy.get()
        }
        pub fn preview_gpu(
            &self,
            branch: &str,
            revision: &str,
        ) -> std::result::Result<js_sys::Promise, JsValue> {
            if self.busy.replace(true) {
                return Err(js("one GPU preview per agent session"));
            }
            let guard = PreviewGuard(self.busy.clone());
            self.cancelled
                .store(false, std::sync::atomic::Ordering::Release);
            let (snapshot, settings) = self
                .session
                .borrow()
                .render_input(&agent_principal(), branch, revision)
                .map_err(js)?;
            let session = self.session.clone();
            let cancelled = self.cancelled.clone();
            let branch = branch.to_owned();
            // Admission and cancellation registration happen before returning
            // the Promise, so an immediate cancellation cannot be lost.
            Ok(future_to_promise(async move {
                let _guard = guard;
                let scene = Evaluator::default().evaluate(&snapshot).map_err(js)?;
                let mut gpu = render_gpu::Renderer::new().await.map_err(js)?;
                let image = gpu
                    .render(&scene, &settings, 0, &cancelled)
                    .await
                    .map_err(js)?;
                if cancelled.load(std::sync::atomic::Ordering::Acquire) {
                    return Err(js("cancelled: preview discarded"));
                }
                let result = session
                    .borrow_mut()
                    .record_render(&agent_principal(), &branch, &image)
                    .map_err(js)?;
                Ok(JsValue::from_str(
                    &serde_json::to_string(&result).map_err(js)?,
                ))
            }))
        }
        /// A pinned animated preview does not qualify a static variant for commit.
        pub fn preview_frame_gpu(
            &self,
            branch: &str,
            request_json: &str,
        ) -> std::result::Result<js_sys::Promise, JsValue> {
            if request_json.len() > 1024 * 1024 {
                return Err(js("budget: frame request exceeds 1 MiB"));
            }
            let request: sequence::FrameRequest = serde_json::from_str(request_json).map_err(js)?;
            if self.busy.replace(true) {
                return Err(js("one GPU preview per agent session"));
            }
            let guard = PreviewGuard(self.busy.clone());
            self.cancelled
                .store(false, std::sync::atomic::Ordering::Release);
            let (snapshot, _) = self
                .session
                .borrow()
                .render_input(&agent_principal(), branch, &request.revision)
                .map_err(js)?;
            request.validate(&snapshot).map_err(js)?;
            let session = self.session.clone();
            let cancelled = self.cancelled.clone();
            let branch = branch.to_owned();
            Ok(future_to_promise(async move {
                let _guard = guard;
                if cancelled.load(std::sync::atomic::Ordering::Acquire) {
                    return Err(js("cancelled: frame discarded"));
                }
                let mut gpu = render_gpu::Renderer::new().await.map_err(js)?;
                let frame = gpu
                    .render_frame(&snapshot, &request, &cancelled)
                    .await
                    .map_err(js)?;
                session
                    .borrow()
                    .render_input(&agent_principal(), &branch, &request.revision)
                    .map_err(js)?;
                Ok(JsValue::from_str(&frame_json(&frame).map_err(js)?))
            }))
        }
        /// Await the consumer after each completed frame. The consumer owns
        /// artifact publication; rejection leaves explicitly partial delivery.
        pub fn preview_sequence_gpu(
            &self,
            branch: &str,
            request_json: &str,
            emit: js_sys::Function,
        ) -> std::result::Result<js_sys::Promise, JsValue> {
            if request_json.len() > 1024 * 1024 {
                return Err(js("budget: sequence request exceeds 1 MiB"));
            }
            let request: sequence::SequenceRequest =
                serde_json::from_str(request_json).map_err(js)?;
            if self.busy.replace(true) {
                return Err(js("one GPU preview per agent session"));
            }
            let guard = PreviewGuard(self.busy.clone());
            self.cancelled
                .store(false, std::sync::atomic::Ordering::Release);
            let (snapshot, _) = self
                .session
                .borrow()
                .render_input(&agent_principal(), branch, &request.revision)
                .map_err(js)?;
            request.validate(&snapshot).map_err(js)?;
            let session = self.session.clone();
            let cancelled = self.cancelled.clone();
            let branch = branch.to_owned();
            Ok(future_to_promise(async move {
                let _guard = guard;
                let mut delivered = 0usize;
                let mut acknowledged = 0usize;
                let result: std::result::Result<JsValue, JsValue> = async {
                    let check = || -> std::result::Result<(), JsValue> {
                        if cancelled.load(std::sync::atomic::Ordering::Acquire) { return Err(js("cancelled: sequence stopped")); }
                        session.borrow().render_input(&agent_principal(), &branch, &request.revision).map_err(js)?;
                        Ok(())
                    };
                    check()?;
                    let mut gpu = render_gpu::Renderer::new().await.map_err(js)?;
                    let mut receipts = vec![];
                    for (index, &time) in request.times.iter().enumerate() {
                        check()?;
                        let frame = gpu.render_frame(&snapshot, &sequence::FrameRequest {
                            revision: request.revision.clone(), clip: request.clip, time, shutter: request.shutter.clone(),
                        }, &cancelled).await.map_err(js)?;
                        check()?;
                        let data = frame_json(&frame).map_err(js)?;
                        delivered += 1;
                        let accepted = emit.call2(&JsValue::UNDEFINED, &JsValue::from_f64(index as f64), &JsValue::from_str(&data))?;
                        JsFuture::from(js_sys::Promise::resolve(&accepted)).await?;
                        acknowledged += 1;
                        receipts.push(frame.image.receipt);
                        check()?;
                    }
                    Ok(JsValue::from_str(&serde_json::json!({"status":"complete","authored_revision":request.revision,"frames":acknowledged,"receipts":receipts}).to_string()))
                }.await;
                result.map_err(|error| js(serde_json::json!({"status":"partial","delivered_frames":delivered,"acknowledged_frames":acknowledged,"error":error.as_string().unwrap_or_else(|| format!("{error:?}"))})))
            }))
        }
        /// Web Lock plus compare-and-publish. Null expected_document_digest creates a
        /// new named project; overwrite requires its exact stored document digest.
        pub async fn save(
            &self,
            name: &str,
            expected_document_digest: Option<String>,
            abort: bool,
        ) -> std::result::Result<String, JsValue> {
            agent_name(name)?;
            let document = self.session.borrow().document().clone();
            let revision = document.snapshot().revision().map_err(js)?;
            let document_digest = digest(&canonical(&document).map_err(js)?);
            let name = format!("agent-{name}.json");
            let root = root().await?;
            let callback =
                Closure::<dyn FnMut(JsValue) -> js_sys::Promise>::new(move |_: JsValue| {
                    let root = root.clone();
                    let name = name.clone();
                    let doc = document.clone();
                    let expected = expected_document_digest.clone();
                    future_to_promise(async move {
                        match file(&root, &name, false).await {
                            Ok(_) => {
                                let saved = load(&root, &name).await?;
                                if expected.as_deref()
                                    != Some(digest(&canonical(&saved).map_err(js)?).as_str())
                                {
                                    return Err(js("conflict: stored project state changed"));
                                }
                            }
                            Err(error) => {
                                let kind =
                                    js_sys::Reflect::get(&error, &JsValue::from_str("name"))?
                                        .as_string();
                                if kind.as_deref() != Some("NotFoundError") {
                                    return Err(error);
                                }
                                if expected.is_some() {
                                    return Err(js("conflict: expected project is missing"));
                                }
                            }
                        }
                        let envelope = Envelope::new(0, None, &doc).map_err(js)?;
                        write(&root, &name, &canonical(&envelope).map_err(js)?, abort).await?;
                        Ok(JsValue::UNDEFINED)
                    })
                });
            let navigator = web_sys::window().ok_or_else(|| js("window"))?.navigator();
            let locks = js_sys::Reflect::get(&navigator, &JsValue::from_str("locks"))?;
            let request: js_sys::Function =
                js_sys::Reflect::get(&locks, &JsValue::from_str("request"))?.dyn_into()?;
            let promise: js_sys::Promise = request
                .call2(
                    &locks,
                    &JsValue::from_str("render-agent-storage-v0"),
                    callback.as_ref(),
                )?
                .dyn_into()?;
            JsFuture::from(promise).await?;
            drop(callback);
            Ok(serde_json::json!({"revision":revision,"document_digest":document_digest,"published":!abort,"storage_class":"OPFS close + Web Lock; browser-managed persistence"}).to_string())
        }
        pub async fn load(name: &str) -> std::result::Result<BrowserAgent, JsValue> {
            agent_name(name)?;
            let root = root().await?;
            Ok(Self::from_document(
                load(&root, &format!("agent-{name}.json")).await?,
            ))
        }
    }
    impl BrowserAgent {
        fn from_document(document: Document) -> Self {
            Self {
                session: std::rc::Rc::new(std::cell::RefCell::new(agent::Session::new(document))),
                cancelled: std::sync::Arc::new(AtomicBool::new(false)),
                busy: std::rc::Rc::new(std::cell::Cell::new(false)),
            }
        }
    }

    /// General browser API; the conformance UI is only one client of this core.
    #[wasm_bindgen]
    pub struct BrowserDocument {
        document: Document,
        candidates: std::collections::BTreeMap<u32, Candidate>,
        next_candidate: u32,
    }
    #[wasm_bindgen]
    impl BrowserDocument {
        #[wasm_bindgen(constructor)]
        pub fn new(id: &str) -> std::result::Result<BrowserDocument, JsValue> {
            let id: Id = serde_json::from_str(&format!("\"{id}\"")).map_err(js)?;
            Ok(Self {
                document: Document::new(Snapshot::empty(id)).map_err(js)?,
                candidates: std::collections::BTreeMap::new(),
                next_candidate: 1,
            })
        }
        pub fn import_json(json: &str) -> std::result::Result<BrowserDocument, JsValue> {
            if json.len() > 128 * 1024 * 1024 {
                return Err(js("document exceeds browser import profile"));
            }
            let document: Document = serde_json::from_str(json).map_err(js)?;
            document.snapshot().validate().map_err(js)?;
            Ok(Self {
                document,
                candidates: std::collections::BTreeMap::new(),
                next_candidate: 1,
            })
        }
        pub fn export_json(&self) -> std::result::Result<String, JsValue> {
            String::from_utf8(canonical(&self.document).map_err(js)?).map_err(js)
        }
        pub fn revision(&self) -> std::result::Result<String, JsValue> {
            self.document.snapshot().revision().map_err(js)
        }
        pub fn execute(&mut self, json: &str) -> std::result::Result<String, JsValue> {
            let result = self
                .document
                .execute_wire(
                    &Principal {
                        id: "local-browser".into(),
                        can_write: true,
                    },
                    json.as_bytes(),
                )
                .map_err(js)?;
            String::from_utf8(result).map_err(js)
        }
        pub fn prepare(&mut self, json: &str) -> std::result::Result<u32, JsValue> {
            if self.candidates.len() >= 16 || json.len() > 16 * 1024 * 1024 {
                return Err(js("candidate or request budget exceeded"));
            }
            let request: Request = serde_json::from_str(json).map_err(js)?;
            let candidate = self
                .document
                .prepare(
                    &Principal {
                        id: "local-browser".into(),
                        can_write: true,
                    },
                    &request,
                )
                .map_err(js)?;
            let id = self.next_candidate;
            self.next_candidate = self
                .next_candidate
                .checked_add(1)
                .ok_or_else(|| js("candidate ID exhausted"))?;
            self.candidates.insert(id, candidate);
            Ok(id)
        }
        pub fn inspect_candidate(&self, id: u32) -> std::result::Result<String, JsValue> {
            let c = self
                .candidates
                .get(&id)
                .ok_or_else(|| js("candidate not found"))?;
            serde_json::to_string(c.receipt()).map_err(js)
        }
        pub fn commit(&mut self, id: u32) -> std::result::Result<String, JsValue> {
            let c = self
                .candidates
                .remove(&id)
                .ok_or_else(|| js("candidate not found"))?;
            serde_json::to_string(&self.document.commit(c).map_err(js)?).map_err(js)
        }
        pub fn discard(&mut self, id: u32) -> bool {
            self.candidates.remove(&id).is_some()
        }
        pub fn branch(&self) -> BrowserDocument {
            Self {
                document: self.document.clone(),
                candidates: std::collections::BTreeMap::new(),
                next_candidate: 1,
            }
        }
    }
    #[wasm_bindgen]
    pub async fn render_document(
        document_json: &str,
        settings_json: &str,
    ) -> std::result::Result<String, JsValue> {
        let document = BrowserDocument::import_json(document_json)?;
        let settings: Settings = serde_json::from_str(settings_json).map_err(js)?;
        let scene = Evaluator::default()
            .evaluate(document.document.snapshot())
            .map_err(js)?;
        let mut gpu = render_gpu::Renderer::new().await.map_err(js)?;
        let image = gpu
            .render(&scene, &settings, 0, &AtomicBool::new(false))
            .await
            .map_err(js)?;
        serde_json::to_string(&serde_json::json!({"receipt":image.receipt,"linear_rgb":image.linear,"depth_meters":image.depth,"normals_world":image.normals,"object_ids":image.objects})).map_err(js)
    }
    async fn root() -> std::result::Result<FileSystemDirectoryHandle, JsValue> {
        let window = web_sys::window().ok_or_else(|| js("window unavailable"))?;
        JsFuture::from(window.navigator().storage().get_directory())
            .await?
            .dyn_into()
    }
    async fn file(
        root: &FileSystemDirectoryHandle,
        name: &str,
        create: bool,
    ) -> std::result::Result<FileSystemFileHandle, JsValue> {
        let options = FileSystemGetFileOptions::new();
        options.set_create(create);
        JsFuture::from(root.get_file_handle_with_options(name, &options))
            .await?
            .dyn_into()
    }
    async fn read(
        root: &FileSystemDirectoryHandle,
        name: &str,
    ) -> std::result::Result<Vec<u8>, JsValue> {
        let handle = file(root, name, false).await?;
        let blob: web_sys::File = JsFuture::from(handle.get_file()).await?.dyn_into()?;
        if blob.size() > 64. * 1024. * 1024. {
            return Err(js("file exceeds browser profile"));
        }
        let buffer = JsFuture::from(blob.array_buffer()).await?;
        Ok(js_sys::Uint8Array::new(&buffer).to_vec())
    }
    async fn write(
        root: &FileSystemDirectoryHandle,
        name: &str,
        bytes: &[u8],
        abort: bool,
    ) -> std::result::Result<(), JsValue> {
        let handle = file(root, name, true).await?;
        let stream: FileSystemWritableFileStream =
            JsFuture::from(handle.create_writable()).await?.dyn_into()?;
        JsFuture::from(stream.write_with_u8_array(bytes)?).await?;
        if abort {
            JsFuture::from(stream.abort()).await?;
        } else {
            JsFuture::from(stream.close()).await?;
        }
        Ok(())
    }
    async fn load(
        root: &FileSystemDirectoryHandle,
        name: &str,
    ) -> std::result::Result<Document, JsValue> {
        let bytes = read(root, name).await?;
        let e: Envelope = serde_json::from_slice(&bytes).map_err(js)?;
        e.decode().map_err(js)
    }
    #[wasm_bindgen]
    pub async fn storage_quota_probe() -> std::result::Result<String, JsValue> {
        let root = root().await?;
        let before = read(&root, "render-conformance-v0.json").await?;
        let failed = write(&root, "render-quota-probe.bin", &vec![0; 128 * 1024], false).await;
        let error = failed
            .err()
            .ok_or_else(|| js("expected configured browser quota failure"))?;
        let name = js_sys::Reflect::get(&error, &JsValue::from_str("name"))?
            .as_string()
            .unwrap_or_default();
        if name != "QuotaExceededError" {
            return Err(js(format!("unexpected storage failure: {error:?}")));
        }
        let after = read(&root, "render-conformance-v0.json").await?;
        if before != after {
            return Err(js("quota failure altered committed document"));
        }
        Ok(serde_json::json!({"error":name,"committed_root_unchanged":true}).to_string())
    }
    thread_local! {static INTERRUPTED_STREAM: std::cell::RefCell<Option<FileSystemWritableFileStream>>=const {std::cell::RefCell::new(None)};}
    #[wasm_bindgen]
    pub async fn stage_interrupted_write() -> std::result::Result<String, JsValue> {
        let root = root().await?;
        let doc = fixtures::demo().map_err(js)?;
        let revision = doc.snapshot().revision().map_err(js)?;
        let envelope = Envelope::new(0, None, &doc).map_err(js)?;
        write(
            &root,
            "render-interrupted-v0.json",
            &canonical(&envelope).map_err(js)?,
            false,
        )
        .await?;
        let handle = file(&root, "render-interrupted-v0.json", false).await?;
        let stream: FileSystemWritableFileStream =
            JsFuture::from(handle.create_writable()).await?.dyn_into()?;
        JsFuture::from(stream.write_with_str("unfinished invalid replacement")?).await?;
        INTERRUPTED_STREAM.with(|slot| *slot.borrow_mut() = Some(stream));
        Ok(revision)
    }
    #[wasm_bindgen]
    pub async fn recover_interrupted_write() -> std::result::Result<String, JsValue> {
        load(&root().await?, "render-interrupted-v0.json")
            .await?
            .snapshot()
            .revision()
            .map_err(js)
    }
    /// Publication is serialized under an origin-scoped Web Lock. close() publishes a
    /// complete checksummed file; browser-managed persistence is not a user backup.
    async fn publish(
        root: FileSystemDirectoryHandle,
        name: String,
        doc: Document,
        abort: bool,
    ) -> std::result::Result<(), JsValue> {
        let callback = Closure::<dyn FnMut(JsValue) -> js_sys::Promise>::new(move |_: JsValue| {
            let root = root.clone();
            let name = name.clone();
            let doc = doc.clone();
            future_to_promise(async move {
                let e = Envelope::new(0, None, &doc).map_err(js)?;
                write(&root, &name, &canonical(&e).map_err(js)?, abort).await?;
                Ok(JsValue::UNDEFINED)
            })
        });
        // This web-sys release gates Web Locks behind unstable bindings; reflect
        // only the two named platform members without executing script strings.
        let navigator = web_sys::window().ok_or_else(|| js("window"))?.navigator();
        let locks = js_sys::Reflect::get(&navigator, &JsValue::from_str("locks"))?;
        let request: js_sys::Function =
            js_sys::Reflect::get(&locks, &JsValue::from_str("request"))?.dyn_into()?;
        let promise: js_sys::Promise = request
            .call2(
                &locks,
                &JsValue::from_str("render-storage-v0"),
                callback.as_ref(),
            )?
            .dyn_into()?;
        JsFuture::from(promise).await?;
        drop(callback);
        Ok(())
    }
    fn display(image: &Image) -> std::result::Result<(), JsValue> {
        let doc = web_sys::window().unwrap().document().unwrap();
        let canvas: web_sys::HtmlCanvasElement = doc
            .get_element_by_id("image")
            .ok_or_else(|| js("canvas"))?
            .dyn_into()?;
        canvas.set_width(image.width);
        canvas.set_height(image.height);
        let ctx: web_sys::CanvasRenderingContext2d = canvas
            .get_context("2d")?
            .ok_or_else(|| js("2d context"))?
            .dyn_into()?;
        let rgba = image
            .linear
            .iter()
            .flat_map(|p| {
                [p[0], p[1], p[2]]
                    .map(|x| (linear_to_srgb(x).clamp(0., 1.) * 255. + 0.5) as u8)
                    .into_iter()
                    .chain([255])
            })
            .collect::<Vec<_>>();
        let image = web_sys::ImageData::new_with_u8_clamped_array_and_sh(
            Clamped(&rgba),
            image.width,
            image.height,
        )?;
        ctx.put_image_data(&image, 0., 0.)
    }
    #[wasm_bindgen]
    pub async fn verify() -> std::result::Result<String, JsValue> {
        let conformance = fixtures::conformance().map_err(js)?;
        let doc = fixtures::demo().map_err(js)?;
        let original = doc.snapshot().revision().map_err(js)?;
        let root = root().await?;
        publish(
            root.clone(),
            "render-conformance-v0.json".into(),
            doc.clone(),
            false,
        )
        .await?;
        let loaded = load(&root, "render-conformance-v0.json").await?;
        if loaded.snapshot().revision().map_err(js)? != original {
            return Err(js("browser document roundtrip mismatch"));
        }
        let mut branch = doc.clone();
        let request = fixtures::request(
            &branch,
            "browser:rename:01",
            vec![Command::Rename {
                entity: Id(4),
                name: "browser candidate".into(),
            }],
        )
        .map_err(js)?;
        branch
            .execute(&fixtures::principal(), &request)
            .map_err(js)?;
        publish(
            root.clone(),
            "render-conformance-v0.json".into(),
            branch.clone(),
            true,
        )
        .await?;
        let recovered = load(&root, "render-conformance-v0.json").await?;
        if recovered.snapshot().revision().map_err(js)? != original {
            return Err(js("aborted browser write changed published root"));
        }
        let exported = canonical(&branch).map_err(js)?;
        write(&root, "render-export-v0.json", &exported, false).await?;
        let exported: Document =
            serde_json::from_slice(&read(&root, "render-export-v0.json").await?).map_err(js)?;
        if exported.snapshot().revision().map_err(js)?
            != branch.snapshot().revision().map_err(js)?
        {
            return Err(js("export/reload mismatch"));
        }
        let mut settings = fixtures::settings();
        settings.width = 48;
        settings.height = 32;
        settings.samples = 8;
        let scene = Evaluator::default().evaluate(doc.snapshot()).map_err(js)?;
        let cpu = render(&scene, &settings, || false).map_err(js)?;
        let mut gpu = render_gpu::Renderer::new().await.map_err(js)?;
        let image = gpu
            .render(&scene, &settings, 0, &AtomicBool::new(false))
            .await
            .map_err(js)?;
        let squared = cpu
            .linear
            .iter()
            .flatten()
            .zip(image.linear.iter().flatten())
            .map(|(a, b)| f64::from((a - b) * (a - b)))
            .sum::<f64>();
        let rmse = (squared / (cpu.linear.len() * 3) as f64).sqrt();
        if rmse > 0.035 {
            return Err(js(format!("browser CPU/GPU RMSE {rmse}")));
        }
        display(&image)?;
        let cancelled = gpu
            .render(&scene, &settings, 0, &AtomicBool::new(true))
            .await
            .unwrap_err()
            .code
            == "cancelled";
        let mut limited = settings.clone();
        limited.max_bytes = 1;
        let budget = gpu
            .render(&scene, &limited, 0, &AtomicBool::new(false))
            .await
            .unwrap_err()
            .code
            == "budget";
        let mut progressive = render_gpu::Progressive::default();
        let mut batch = settings.clone();
        batch.samples = 4;
        progressive
            .step(&mut gpu, &scene, &batch, &AtomicBool::new(false))
            .await
            .map_err(js)?;
        let total = progressive
            .step(&mut gpu, &scene, &batch, &AtomicBool::new(false))
            .await
            .map_err(js)?;
        let progressive_error = total
            .linear
            .iter()
            .flatten()
            .zip(image.linear.iter().flatten())
            .map(|(a, b)| (a - b).abs())
            .fold(0f32, f32::max);
        if progressive_error > 1e-5 {
            return Err(js("browser progressive accumulation mismatch"));
        }
        let allocation_rejected = gpu.allocation_fault_probe().await.map_err(js)?;
        if !allocation_rejected {
            return Err(js("over-limit GPU buffer was not rejected"));
        }
        let raster = gpu.raster_preview(&scene, &settings).await.map_err(js)?;
        if raster.len() != (settings.width * settings.height) as usize
            || raster.iter().all(|p| p == &raster[0])
        {
            return Err(js("browser raster coverage failed"));
        }
        let capabilities = gpu.capabilities.clone();
        let cancellation_start = js_sys::Date::now();
        let cancelled_after_submit = gpu
            .cancellation_fault_probe(&scene, &settings)
            .await
            .map_err(js)?;
        let cancelled_render_ms = js_sys::Date::now() - cancellation_start;
        let (analytic_scene, analytic_settings, expected) =
            fixtures::diffuse_plane().map_err(js)?;
        let analytic = gpu
            .render(
                &analytic_scene,
                &analytic_settings,
                0,
                &AtomicBool::new(false),
            )
            .await
            .map_err(js)?;
        let analytic_error = analytic
            .linear
            .iter()
            .flat_map(|p| p.iter().zip(expected).map(|(a, b)| (a - b).abs()))
            .fold(0f32, f32::max);
        if analytic_error > 1e-6
            || analytic.objects.iter().any(Option::is_none)
            || !cancelled_after_submit
        {
            return Err(js("analytic plane or submitted cancellation failed"));
        }
        gpu.destroy();
        let lost = gpu
            .render(&scene, &settings, 0, &AtomicBool::new(false))
            .await
            .unwrap_err()
            .code
            == "device_lost";
        let mut rebuilt = render_gpu::Renderer::new().await.map_err(js)?;
        let rebuilt = rebuilt
            .render(&scene, &settings, 0, &AtomicBool::new(false))
            .await
            .map_err(js)?;
        let equal = rebuilt.linear == image.linear;
        if !cancelled || !budget || !lost || !equal {
            return Err(js("browser GPU fault conformance failed"));
        }
        let worker = worker_conformance().await?;
        let report = serde_json::json!({"status":"passed","gpu_over_limit_allocation_rejected":allocation_rejected,"cancel_after_submit":cancelled_after_submit,"cancelled_render_ms_including_pack_submit_drain":cancelled_render_ms,"analytic_plane_max_error":analytic_error,"analytic_plane_tolerance":1e-6,"core":conformance,"revision":original,"opfs_roundtrip":true,"aborted_write_preserves_root":true,"local_export_roundtrip":true,"browser_storage_class":"origin-managed; atomic close; user export required for backup","cpu_gpu_rmse":rmse,"progressive_max_error":progressive_error,"cancel_before_submit":cancelled,"budget_rejection":budget,"device_destroy_rejected":lost,"device_recreated_equal":equal,"device":capabilities,"shared_memory_required":false,"raster_coverage":true,"worker":worker});
        serde_json::to_string_pretty(&report).map_err(js)
    }
    #[wasm_bindgen(start)]
    pub fn start() {
        if web_sys::window().is_none() {
            spawn_local(async {
                let scope: web_sys::DedicatedWorkerGlobalScope = js_sys::global().unchecked_into();
                let result = worker_run().await;
                let text = match result {
                    Ok(text) => text,
                    Err(error) => {
                        serde_json::json!({"status":"failed","error":format!("{error:?}")})
                            .to_string()
                    }
                };
                let _ = scope.post_message(&JsValue::from_str(&text));
            });
            return;
        }
        spawn_local(async {
            let result = verify().await;
            let doc = web_sys::window().unwrap().document().unwrap();
            if let Some(report) = doc.get_element_by_id("report") {
                match result {
                    Ok(text) => {
                        report.set_text_content(Some(&text));
                        let _ = report.set_attribute("data-status", "passed");
                    }
                    Err(error) => {
                        report.set_text_content(Some(&format!("{error:?}")));
                        let _ = report.set_attribute("data-status", "failed");
                    }
                }
            }
        });
    }
    async fn worker_run() -> std::result::Result<String, JsValue> {
        let core = fixtures::conformance().map_err(js)?;
        let doc = fixtures::demo().map_err(js)?;
        let scene = Evaluator::default().evaluate(doc.snapshot()).map_err(js)?;
        let mut s = fixtures::settings();
        s.width = 16;
        s.height = 12;
        s.samples = 2;
        let cpu = render(&scene, &s, || false).map_err(js)?;
        let mut gpu = render_gpu::Renderer::new().await.map_err(js)?;
        let image = gpu
            .render(&scene, &s, 0, &AtomicBool::new(false))
            .await
            .map_err(js)?;
        let error = cpu
            .linear
            .iter()
            .flatten()
            .zip(image.linear.iter().flatten())
            .map(|(a, b)| (a - b).abs())
            .fold(0f32, f32::max);
        if error > 0.01 {
            return Err(js("worker CPU/GPU mismatch"));
        }
        Ok(serde_json::json!({"status":"passed","core":core,"cpu_gpu_max_error":error,"shared_memory":false,"execution":"dedicated module worker"}).to_string())
    }
    async fn worker_conformance() -> std::result::Result<serde_json::Value, JsValue> {
        let options = web_sys::WorkerOptions::new();
        options.set_type(web_sys::WorkerType::Module);
        let worker = web_sys::Worker::new_with_options("pkg/bootstrap.js", &options)?;
        let state = std::rc::Rc::new(std::cell::RefCell::new((
            None::<String>,
            None::<std::task::Waker>,
        )));
        let received = state.clone();
        let callback = Closure::<dyn FnMut(web_sys::MessageEvent)>::new(
            move |event: web_sys::MessageEvent| {
                let mut s = received.borrow_mut();
                s.0 = event.data().as_string();
                if let Some(w) = s.1.take() {
                    w.wake();
                }
            },
        );
        worker.set_onmessage(Some(callback.as_ref().unchecked_ref()));
        let text = std::future::poll_fn(|cx| {
            let mut s = state.borrow_mut();
            if let Some(text) = s.0.take() {
                std::task::Poll::Ready(text)
            } else {
                s.1 = Some(cx.waker().clone());
                std::task::Poll::Pending
            }
        })
        .await;
        worker.terminate();
        worker.set_onmessage(None);
        drop(callback);
        let value: serde_json::Value = serde_json::from_str(&text).map_err(js)?;
        if value["status"] != "passed" {
            return Err(js(text));
        }
        Ok(value)
    }
}
