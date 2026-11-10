use crate::{
    protect,
    config::{self, Settings},
    images::{self, Block},
    models::{self, Offline},
    process,
    providers::Providers,
};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    os::unix::{
        fs::{OpenOptionsExt, PermissionsExt},
        io::AsRawFd,
    },
    path::PathBuf,
    sync::Arc,
    time::Instant,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
    sync::{Mutex, watch},
};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub v: u8,
    pub event: String,
    pub job: u64,
    pub mode: String,
    pub status: String,
    pub source_kind: String,
    pub source_text: String,
    pub translated: String,
    pub source_language: String,
    pub target_language: String,
    pub original_image: String,
    pub translated_image: String,
    pub image_width: u32,
    pub image_height: u32,
    pub blocks: Vec<Block>,
    pub error: String,
    pub notice: String,
    pub elapsed_ms: u128,
    pub settings: Settings,
    pub models: Vec<Value>,
    pub download: Value,
    pub offline_engine: bool,
    pub languages: Value,
    pub ocr_installed: Vec<String>,
    pub history: Vec<Value>,
    pub hide_serial: u64,
}
impl Default for Snapshot {
    fn default() -> Self {
        Self {
            v: 1,
            event: "state".into(),
            job: 0,
            mode: "text".into(),
            status: "idle".into(),
            source_kind: String::new(),
            source_text: String::new(),
            translated: String::new(),
            source_language: String::new(),
            target_language: String::new(),
            original_image: String::new(),
            translated_image: String::new(),
            image_width: 0,
            image_height: 0,
            blocks: vec![],
            error: String::new(),
            notice: String::new(),
            elapsed_ms: 0,
            settings: Settings::load(),
            models: models::status(),
            download: Value::Null,
            offline_engine: false,
            languages: crate::languages::catalog(),
            ocr_installed: vec![],
            history: vec![],
            hide_serial: 0,
        }
    }
}
struct State {
    snapshot: Snapshot,
    cancel: CancellationToken,
    files: Option<Arc<tempfile::TempDir>>,
    image: Option<Arc<image::DynamicImage>>,
    original_path: Option<PathBuf>,
    download_cancel: CancellationToken,
}
#[derive(Clone)]
struct Job {
    id: u64,
    cancel: CancellationToken,
    files: Arc<tempfile::TempDir>,
    settings: Settings,
    started: Instant,
}
pub struct App {
    state: Mutex<State>,
    updates: watch::Sender<Snapshot>,
    providers: Arc<Providers>,
    offline: Mutex<Offline>,
}
impl App {
    pub fn new() -> Result<Arc<Self>> {
        let mut snapshot = Snapshot::default();
        if snapshot.settings.history {
            snapshot.history = std::fs::read(config::data_dir().join("history.json"))
                .ok()
                .and_then(|b| serde_json::from_slice(&b).ok())
                .unwrap_or_default();
        }
        let (updates, _) = watch::channel(snapshot.clone());
        Ok(Arc::new(Self {
            state: Mutex::new(State {
                snapshot,
                cancel: CancellationToken::new(),
                files: None,
                image: None,
                original_path: None,
                download_cancel: CancellationToken::new(),
            }),
            updates,
            providers: Arc::new(Providers::new()?),
            offline: Mutex::new(Offline::new()),
        }))
    }
    fn publish(&self, state: &State) {
        self.updates.send_replace(state.snapshot.clone());
    }
    async fn begin(&self, mode: &str, keep_image: bool) -> Result<Job> {
        let mut s = self.state.lock().await;
        s.cancel.cancel();
        s.cancel = CancellationToken::new();
        s.snapshot.job += 1;
        s.snapshot.mode = mode.into();
        s.snapshot.source_kind = "input".into();
        s.snapshot.status = "working".into();
        s.snapshot.error.clear();
        s.snapshot.notice.clear();
        s.snapshot.translated.clear();
        s.snapshot.elapsed_ms = 0;
        if !keep_image {
            s.files = Some(Arc::new(
                tempfile::Builder::new().prefix("omatranslate-").tempdir()?,
            ));
            s.image = None;
            s.original_path = None;
            s.snapshot.blocks.clear();
            s.snapshot.original_image.clear();
            s.snapshot.translated_image.clear();
            s.snapshot.source_text.clear();
            s.snapshot.image_width = 0;
            s.snapshot.image_height = 0;
        }
        let job = Job {
            id: s.snapshot.job,
            cancel: s.cancel.clone(),
            files: s.files.as_ref().context("invalidImage")?.clone(),
            settings: s.snapshot.settings.clone(),
            started: Instant::now(),
        };
        self.publish(&s);
        Ok(job)
    }
    async fn update(&self, job: &Job, f: impl FnOnce(&mut State)) {
        let mut s = self.state.lock().await;
        if s.snapshot.job != job.id || job.cancel.is_cancelled() {
            return;
        }
        f(&mut s);
        self.publish(&s);
    }
    async fn fail(&self, job: &Job, error: anyhow::Error) {
        if job.cancel.is_cancelled() {
            return;
        }
        let code = error_code(&error);
        self.update(job, |s| {
            s.snapshot.status = "error".into();
            s.snapshot.error = code;
        })
        .await;
    }
    async fn show(&self, settings: bool) {
        if std::env::var_os("OMATRANSLATE_NO_SHELL").is_some() {
            return;
        }
        let payload = if settings {
            r#"{"settings":true}"#
        } else {
            "{}"
        };
        let result = process::run(
            "omarchy-shell",
            &["shell", "summon", "vornashev.omatranslate", payload],
            &[],
            3,
            &CancellationToken::new(),
        )
        .await;
        if result.is_err() {
            let mut s = self.state.lock().await;
            s.snapshot.notice = "shellUnavailable".into();
            self.publish(&s);
        }
    }
    pub async fn request(self: &Arc<Self>, request: Value) -> Result<Value> {
        if request["v"].as_u64() != Some(1) {
            bail!("protocolVersion");
        }
        let op = request["op"].as_str().context("invalidRequest")?;
        match op {
            "subscribe" | "status" => {
                return Ok(serde_json::to_value(&self.state.lock().await.snapshot)?);
            }
            "open" | "settings" => {
                self.show(op == "settings").await;
            }
            "replace" => {
                // Translate the selection and type the result over it, without the window.
                let app = self.clone();
                let job = self.begin("text", false).await?;
                let id = job.id;
                tokio::spawn(async move {
                    if let Err(e) = app.replace(&job).await {
                        let code = error_code(&e);
                        app.fail(&job, e).await;
                        notify_failure(&code).await;
                    }
                });
                return Ok(json!({"v":1,"event":"ack","job":id}));
            }
            "selection" | "clipboard" | "screenshot" | "image" => {
                let mode = op.to_owned();
                let path = request["path"].as_str().map(PathBuf::from);
                let app = self.clone();
                let job = self
                    .begin(
                        if op == "screenshot" || op == "image" {
                            "image"
                        } else {
                            "text"
                        },
                        false,
                    )
                    .await?;
                let id = job.id;
                tokio::spawn(async move {
                    if let Err(e) = app.capture(&job, &mode, path).await {
                        app.fail(&job, e).await;
                        app.show(false).await;
                    }
                });
                return Ok(json!({"v":1,"event":"ack","job":id}));
            }
            "translate" => {
                let text = request["text"]
                    .as_str()
                    .context("invalidRequest")?
                    .to_owned();
                if text.len() > 100000 {
                    bail!("inputTooLarge");
                }
                let job = self.begin("text", false).await?;
                let id = job.id;
                let app = self.clone();
                tokio::spawn(async move {
                    if let Err(e) = app.text(&job, text, "input").await {
                        app.fail(&job, e).await;
                    }
                });
                return Ok(json!({"v":1,"event":"ack","job":id}));
            }
            "retranslate" | "editBlock" => {
                let image = self.state.lock().await.snapshot.mode == "image";
                if image {
                    let index = request["index"].as_u64();
                    let source = request["text"].as_str().unwrap_or("").to_owned();
                    if source.len() > 10000 {
                        bail!("inputTooLarge");
                    }
                    let job = self.begin("image", true).await?;
                    self.update(&job, |s| {
                        if let Some(i) = index
                            && let Some(b) = s.snapshot.blocks.get_mut(i as usize)
                        {
                            b.source = source;
                        }
                    })
                    .await;
                    let app = self.clone();
                    tokio::spawn(async move {
                        if let Err(e) = app.translate_image(&job).await {
                            app.fail(&job, e).await;
                        }
                    });
                } else {
                    let text = self.state.lock().await.snapshot.source_text.clone();
                    let job = self.begin("text", false).await?;
                    let app = self.clone();
                    tokio::spawn(async move {
                        if let Err(e) = app.text(&job, text, "input").await {
                            app.fail(&job, e).await;
                        }
                    });
                }
            }
            "setSettings" => {
                let mut s = self.state.lock().await;
                let mut value = serde_json::to_value(&s.snapshot.settings)?;
                for (key, v) in request["settings"].as_object().context("invalidRequest")? {
                    value[key] = v.clone();
                }
                let mut settings: Settings =
                    serde_json::from_value(value).context("invalidSettings")?;
                settings.validate()?;
                settings.save()?;
                s.cancel.cancel();
                s.snapshot.status = "idle".into();
                s.snapshot.settings = settings.clone();
                if !settings.history {
                    s.snapshot.history.clear();
                    let _ = std::fs::remove_file(config::data_dir().join("history.json"));
                }
                self.publish(&s);
                drop(s);
                let app = self.clone();
                tokio::spawn(async move {
                    app.warm_models().await;
                });
            }
            "dismiss" => {
                let mut s = self.state.lock().await;
                s.cancel.cancel();
                s.files = None;
                s.image = None;
                s.snapshot.status = "idle".into();
                // Without the image there is nothing to show in image mode: go back
                // to text, or the next open shows an empty canvas.
                s.snapshot.mode = "text".into();
                s.snapshot.image_width = 0;
                s.snapshot.image_height = 0;
                s.snapshot.error.clear();
                s.snapshot.notice.clear();
                s.snapshot.original_image.clear();
                s.snapshot.translated_image.clear();
                s.snapshot.blocks.clear();
                s.snapshot.source_text.clear();
                s.snapshot.translated.clear();
                self.publish(&s);
            }
            "copy" => {
                let (text, path) = {
                    let s = self.state.lock().await;
                    (
                        s.snapshot.translated.clone(),
                        s.snapshot.translated_image.clone(),
                    )
                };
                if request["kind"] == "image" {
                    if path.is_empty() {
                        bail!("invalidImage");
                    }
                    process::run(
                        "wl-copy",
                        &["--type", "image/png"],
                        &tokio::fs::read(path).await?,
                        5,
                        &CancellationToken::new(),
                    )
                    .await?;
                } else {
                    process::run(
                        "wl-copy",
                        &["--type", "text/plain;charset=utf-8"],
                        text.as_bytes(),
                        5,
                        &CancellationToken::new(),
                    )
                    .await?;
                }
                let mut s = self.state.lock().await;
                s.snapshot.notice = "copied".into();
                self.publish(&s);
            }
            "save" => {
                let source = self.state.lock().await.snapshot.translated_image.clone();
                if source.is_empty() {
                    bail!("invalidImage");
                }
                let (path, confirmed) = if let Some(path) = request["path"].as_str() {
                    (PathBuf::from(path), request["overwrite"] == true)
                } else {
                    match crate::portal::save_path().await? {
                        Some(path) => (path, true),
                        None => return Ok(json!({"v":1,"event":"ack"})),
                    }
                };
                if path.exists() && !confirmed {
                    bail!("fileExists");
                }
                if let Some(original) = &self.state.lock().await.original_path
                    && path.canonicalize().ok().as_ref() == Some(original)
                {
                    bail!("originalProtected");
                }
                config::atomic_write(&path, &tokio::fs::read(source).await?)?;
                let mut s = self.state.lock().await;
                s.snapshot.notice = "saved".into();
                self.publish(&s);
            }
            "downloadModel" => {
                let pair = request["pair"]
                    .as_str()
                    .context("invalidRequest")?
                    .to_owned();
                if !models::PAIRS.contains(&pair.as_str()) {
                    bail!("invalidModel");
                }
                let mut s = self.state.lock().await;
                if !s.snapshot.download.is_null() {
                    bail!("downloadBusy");
                }
                s.download_cancel = CancellationToken::new();
                let cancel = s.download_cancel.clone();
                s.snapshot.download = json!({"pair":pair,"received":0,"total":null});
                self.publish(&s);
                drop(s);
                let app = self.clone();
                tokio::spawn(async move {
                    let sender = app.updates.clone();
                    let state = app.clone();
                    let progress_pair = pair.clone();
                    let result = models::download(&pair, &cancel, move |received, total| {
                        if let Ok(mut s) = state.state.try_lock() {
                            s.snapshot.download =
                                json!({"pair":progress_pair,"received":received,"total":total});
                            sender.send_replace(s.snapshot.clone());
                        }
                    })
                    .await;
                    let mut s = app.state.lock().await;
                    s.snapshot.download = Value::Null;
                    s.snapshot.models = models::status();
                    if let Err(e) = result
                        && !cancel.is_cancelled()
                    {
                        s.snapshot.error = error_code(&e);
                    }
                    app.publish(&s);
                });
            }
            "cancelDownload" => {
                self.state.lock().await.download_cancel.cancel();
            }
            "removeModel" => {
                let pair = request["pair"].as_str().context("invalidRequest")?;
                if !models::PAIRS.contains(&pair) {
                    bail!("invalidModel");
                }
                let s = self.state.lock().await;
                if !s.snapshot.download.is_null() {
                    bail!("downloadBusy");
                }
                s.cancel.cancel();
                drop(s);
                self.offline.lock().await.reset();
                if models::model_dir(pair).exists() {
                    tokio::fs::remove_dir_all(models::model_dir(pair)).await?;
                }
                let mut s = self.state.lock().await;
                s.snapshot.models = models::status();
                self.publish(&s);
            }
            "clearHistory" => {
                let mut s = self.state.lock().await;
                s.snapshot.history.clear();
                let _ = std::fs::remove_file(config::data_dir().join("history.json"));
                self.publish(&s);
            }
            "applyShortcuts" => {
                let script = integration_script();
                process::run(
                    "python3",
                    &[script.to_str().context("missingTool")?, "--bindings-only"],
                    &[],
                    10,
                    &CancellationToken::new(),
                )
                .await?;
                let mut s = self.state.lock().await;
                s.snapshot.notice = "shortcutsApplied".into();
                self.publish(&s);
            }
            _ => bail!("unknownOperation"),
        }
        Ok(json!({"v":1,"event":"ack"}))
    }
    async fn capture(self: &Arc<Self>, job: &Job, mode: &str, path: Option<PathBuf>) -> Result<()> {
        self.update(job, |s| {
            s.snapshot.source_kind = mode.into();
            s.snapshot.status = "capturing".into();
            if mode == "screenshot" {
                s.snapshot.hide_serial += 1;
            }
        })
        .await;
        let image_path = job.files.path().join("original.png");
        match mode {
            "selection" => {
                let text = process::run(
                    "wl-paste",
                    &["--primary", "--no-newline", "--type", "text"],
                    &[],
                    2,
                    &job.cancel,
                )
                .await;
                match text {
                    Ok(bytes) if !bytes.is_empty() => {
                        let value = String::from_utf8_lossy(&bytes).into_owned();
                        self.show(false).await;
                        return self.text(job, value, mode).await;
                    }
                    _ => {
                        self.update(job, |s| {
                            s.snapshot.status = "idle".into();
                            s.snapshot.notice = "noSelection".into();
                        })
                        .await;
                        self.show(false).await;
                        return Ok(());
                    }
                }
            }
            "clipboard" => {
                let types = process::run("wl-paste", &["--list-types"], &[], 2, &job.cancel)
                    .await
                    .context("emptyClipboard")?;
                let types = String::from_utf8_lossy(&types);
                let mime = ["image/png", "image/jpeg", "image/webp"]
                    .into_iter()
                    .find(|mime| types.lines().any(|line| line == *mime));
                if let Some(mime) = mime {
                    let bytes =
                        process::run("wl-paste", &["--type", mime], &[], 5, &job.cancel).await?;
                    config::atomic_write(&image_path, &bytes)?;
                } else {
                    let bytes = process::run(
                        "wl-paste",
                        &["--no-newline", "--type", "text"],
                        &[],
                        2,
                        &job.cancel,
                    )
                    .await
                    .context("emptyClipboard")?;
                    self.show(false).await;
                    return self
                        .text(job, String::from_utf8_lossy(&bytes).into_owned(), mode)
                        .await;
                }
            }
            "screenshot" => {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                let geometry = process::run("slurp", &[], &[], 300, &job.cancel).await;
                let geometry = match geometry {
                    Ok(g) if !g.is_empty() => g,
                    _ => {
                        self.update(job, |s| s.snapshot.status = "idle".into())
                            .await;
                        return Ok(());
                    }
                };
                let geometry = String::from_utf8(geometry)?;
                let bytes =
                    process::run("grim", &["-g", geometry.trim(), "-"], &[], 10, &job.cancel)
                        .await?;
                config::atomic_write(&image_path, &bytes)?;
            }
            "image" => {
                let original = path.context("invalidImage")?.canonicalize()?;
                let image = images::read(&original)?;
                self.update(job, |s| s.original_path = Some(original)).await;
                images::save(&image, &image_path)?;
            }
            _ => bail!("invalidRequest"),
        }
        let path = image_path.clone();
        let image = Arc::new(tokio::task::spawn_blocking(move || images::read(&path)).await??);
        self.update(job, |s| {
            s.image = Some(image.clone());
            s.snapshot.mode = "image".into();
            s.snapshot.image_width = image.width();
            s.snapshot.image_height = image.height();
            s.snapshot.original_image = image_path.to_string_lossy().into_owned();
            s.snapshot.status = "ocr".into();
        })
        .await;
        self.show(false).await;
        let blocks = images::recognize(
            &image,
            job.files.path(),
            &job.settings.ocr_languages,
            &job.cancel,
        )
        .await?;
        self.update(job, |s| {
            s.snapshot.source_text = blocks
                .iter()
                .map(|b| b.source.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            s.snapshot.blocks = blocks;
        })
        .await;
        self.translate_image(job).await
    }
    async fn warm_models(&self) {
        let settings = self.state.lock().await.snapshot.settings.clone();
        if !settings.keep_models_warm {
            self.offline.lock().await.reset();
            return;
        }
        let target = &settings.target;
        let installed = |p: &str| models::model_dir(p).join("integrity.json").is_file();
        let pair = models::PAIRS
            .iter()
            .filter(|p| installed(p))
            .filter_map(|p| p.split_once('_'))
            .min_by_key(|(_, to)| to != target);
        if let Some((source, target)) = pair {
            let _ = self
                .offline
                .lock()
                .await
                .translate(&[], source, target, true, &CancellationToken::new())
                .await;
        }
    }
    /// Translates segments, keeping names and glossary terms as they are.
    /// `names` also keeps "Name 5.2"-style product names (screens, not prose).
    async fn translated(&self, job: &Job, segments: &[String], names: bool) -> Result<Vec<String>> {
        let protected: Vec<_> = segments
            .iter()
            .map(|s| protect::protect(s, &job.settings.glossary, names))
            .collect();
        let pending: Vec<String> = protected
            .iter()
            .filter(|p| !p.is_untranslatable())
            .map(|p| p.text.clone())
            .collect();
        let mut translated = if pending.is_empty() {
            Vec::new()
        } else {
            self.translate_raw(job, segments, &pending).await?
        }
        .into_iter();
        Ok(protected
            .iter()
            .map(|p| {
                if p.is_untranslatable() {
                    p.original()
                } else {
                    p.restore(&translated.next().unwrap_or_default())
                }
            })
            .collect())
    }
    async fn translate_raw(&self, job: &Job, original: &[String], segments: &[String]) -> Result<Vec<String>> {
        let (source, target) = job.settings.languages(&original.join(" "));
        self.update(job, |s| {
            s.snapshot.source_language = source.clone();
            s.snapshot.target_language = target.clone();
            s.snapshot.status = "translating".into();
        })
        .await;
        if job.cancel.is_cancelled() {
            bail!("cancelled");
        }
        if job.settings.provider == "local" {
            self.offline
                .lock()
                .await
                .translate(
                    segments,
                    &source,
                    &target,
                    job.settings.keep_models_warm,
                    &job.cancel,
                )
                .await
        } else {
            self.providers
                .translate(segments, &job.settings, &source, &target, &job.cancel)
                .await
        }
    }
    async fn text(&self, job: &Job, text: String, source_kind: &str) -> Result<()> {
        if text.len() > 100000 {
            bail!("inputTooLarge");
        }
        self.update(job, |s| {
            s.snapshot.source_text = text.clone();
            s.snapshot.source_kind = source_kind.into();
        })
        .await;
        if text.trim().is_empty() {
            self.update(job, |s| s.snapshot.status = "idle".into())
                .await;
            return Ok(());
        }
        let output = self
            .translated(job, &[text], false)
            .await?
            .into_iter()
            .next()
            .unwrap_or_default();
        self.update(job, |s| {
            s.snapshot.translated = output;
            s.snapshot.status = "ready".into();
            s.snapshot.elapsed_ms = job.started.elapsed().as_millis();
            record_history(s);
        })
        .await;
        Ok(())
    }
    /// "Translate and replace": types the translation over the selection in the
    /// focused app. Typing replaces the selection and leaves the clipboard alone.
    async fn replace(&self, job: &Job) -> Result<()> {
        let bytes = process::run(
            "wl-paste",
            &["--primary", "--no-newline", "--type", "text"],
            &[],
            2,
            &job.cancel,
        )
        .await
        .unwrap_or_default();
        let text = String::from_utf8_lossy(&bytes).into_owned();
        if text.trim().is_empty() {
            bail!("noSelection");
        }
        if text.len() > 20000 {
            bail!("inputTooLarge");
        }
        self.update(job, |s| {
            s.snapshot.source_text = text.clone();
            s.snapshot.source_kind = "selection".into();
        })
        .await;
        let output = self
            .translated(job, std::slice::from_ref(&text), false)
            .await?
            .into_iter()
            .next()
            .unwrap_or_default();
        if output.trim().is_empty() {
            bail!("providerError");
        }
        // Let the shortcut's modifiers go first, or typed letters become shortcuts.
        tokio::time::sleep(std::time::Duration::from_millis(120)).await;
        if job.cancel.is_cancelled() {
            bail!("cancelled");
        }
        // Keep the selection's trailing space (double-click often selects "word ").
        let trailing = &text[text.trim_end().len()..];
        let typed = format!("{}{}", output.trim_end(), trailing);
        process::run("wtype", &["-"], typed.as_bytes(), 30, &job.cancel).await?;
        self.update(job, |s| {
            s.snapshot.translated = output;
            s.snapshot.status = "ready".into();
            s.snapshot.notice = "replaced".into();
            s.snapshot.elapsed_ms = job.started.elapsed().as_millis();
            record_history(s);
        })
        .await;
        Ok(())
    }
    async fn translate_image(&self, job: &Job) -> Result<()> {
        let (mut blocks, image) = {
            let s = self.state.lock().await;
            (
                s.snapshot.blocks.clone(),
                s.image.clone().context("invalidImage")?,
            )
        };
        let source: Vec<_> = blocks.iter().map(|b| b.source.clone()).collect();
        let translations = self.translated(job, &source, true).await?;
        for (b, t) in blocks.iter_mut().zip(translations) {
            b.translated = t;
        }
        let path = job.files.path().join(format!("translated-{}.png", job.id));
        let saved = path.clone();
        let minimum = job.settings.min_font_size;
        let blocks = tokio::task::spawn_blocking(move || {
            let rendered = images::render(&image, &mut blocks, minimum)?;
            images::save(&rendered, &saved)?;
            Ok::<_, anyhow::Error>(blocks)
        })
        .await??;
        self.update(job, |s| {
            s.snapshot.translated = blocks
                .iter()
                .map(|b| b.translated.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            s.snapshot.blocks = blocks;
            s.snapshot.translated_image = path.to_string_lossy().into_owned();
            s.snapshot.status = "ready".into();
            s.snapshot.elapsed_ms = job.started.elapsed().as_millis();
            record_history(s);
        })
        .await;
        Ok(())
    }
}
/// Replace runs without the window, so failures need a desktop notification.
async fn notify_failure(code: &str) {
    if code == "cancelled" || std::env::var_os("OMATRANSLATE_NO_SHELL").is_some() {
        return;
    }
    let key = match code {
        "noSelection" => "notifyNoSelection",
        "networkError" => "notifyNetworkError",
        "rateLimited" => "notifyRateLimited",
        "missingTool" => "notifyMissingTool",
        _ => "notifyReplaceFailed",
    };
    let message = &ui_string(key);
    let _ = process::run(
        "notify-send",
        &["--app-name=OmaTranslate", "OmaTranslate", message],
        &[],
        3,
        &CancellationToken::new(),
    )
    .await;
}
/// A string from the plugin's i18n catalog in the session language, falling
/// back to the base language, then English (the same order as the QML side).
fn ui_string(key: &str) -> String {
    let dir = config::resource_path("plugin/i18n", "plugin/plugin/i18n");
    // The shell's display language (Setup > Language), as the QML side uses.
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| config::home().join(".config"));
    let locale = std::fs::read_to_string(config.join("omarchy/language"))
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            ["LC_ALL", "LC_MESSAGES", "LANG"]
                .iter()
                .filter_map(|v| std::env::var(v).ok())
                .find(|v| !v.is_empty())
        })
        .unwrap_or_default();
    let full = locale.split(['.', '@']).next().unwrap_or("").replace('_', "-");
    let base = full.split('-').next().unwrap_or("").to_owned();
    [full, base, "en".into()]
        .iter()
        .filter(|code| !code.is_empty() && code.bytes().all(|b| b.is_ascii_alphabetic() || b == b'-'))
        .find_map(|code| {
            let bytes = std::fs::read(dir.join(format!("{code}.json"))).ok()?;
            let catalog: Value = serde_json::from_slice(&bytes).ok()?;
            catalog[key].as_str().filter(|s| !s.is_empty()).map(str::to_owned)
        })
        .unwrap_or_else(|| "Couldn't translate and replace the selection.".into())
}
fn record_history(s: &mut State) {
    if s.snapshot.settings.history {
        s.snapshot.history.insert(0,json!({"source":s.snapshot.source_text,"translated":s.snapshot.translated,"provider":s.snapshot.settings.provider}));
        s.snapshot.history.truncate(100);
        let _ = config::atomic_write(
            &config::data_dir().join("history.json"),
            &serde_json::to_vec(&s.snapshot.history).unwrap_or_default(),
        );
    }
}
fn integration_script() -> PathBuf {
    config::resource_path("bin/integrate.py", "integrate.py")
}
pub fn error_code(error: &anyhow::Error) -> String {
    let code = error.to_string();
    if code.len() < 60 && code.bytes().all(|c| c.is_ascii_alphanumeric()) {
        code
    } else {
        "operationFailed".into()
    }
}
pub async fn frame<R: tokio::io::AsyncBufRead + Unpin>(reader: &mut R) -> Result<Option<Value>> {
    let mut bytes = Vec::new();
    let n = reader
        .take(1024 * 1024 + 1)
        .read_until(b'\n', &mut bytes)
        .await?;
    if n == 0 {
        return Ok(None);
    }
    if n > 1024 * 1024 || !bytes.ends_with(b"\n") {
        bail!("frameTooLarge");
    }
    Ok(Some(
        serde_json::from_slice(&bytes).context("invalidRequest")?,
    ))
}
async fn connection(app: Arc<App>, socket: UnixStream) -> Result<()> {
    let (read, write) = socket.into_split();
    connection_io(app, read, write).await
}
async fn connection_io<R, W>(app: Arc<App>, read: R, mut write: W) -> Result<()>
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
    W: tokio::io::AsyncWrite + Unpin,
{
    let mut reader = BufReader::new(read);
    let mut updates = app.updates.subscribe();
    let mut subscribed = false;
    let (requests, mut incoming) = tokio::sync::mpsc::channel(8);
    let reader_task = tokio::spawn(async move {
        while let Ok(Some(value)) = frame(&mut reader).await {
            if requests.send(value).await.is_err() {
                break;
            }
        }
    });
    struct Abort(tokio::task::JoinHandle<()>);
    impl Drop for Abort {
        fn drop(&mut self) {
            self.0.abort();
        }
    }
    let _reader_guard = Abort(reader_task);
    loop {
        tokio::select! {
            request=incoming.recv()=>{
                let Some(request)=request else{return Ok(());};if request["op"]=="subscribe"{subscribed=true;}
                let response=match app.request(request).await{Ok(v)=>v,Err(e)=>json!({"v":1,"event":"error","error":error_code(&e)})};
                write.write_all(format!("{response}\n").as_bytes()).await?;
            },
            changed=updates.changed(),if subscribed=>{changed?;let value=updates.borrow_and_update().clone();write.write_all(format!("{}\n",serde_json::to_string(&value)?).as_bytes()).await?;}
        }
    }
}
pub async fn serve_stdio() -> Result<()> {
    connection_io(App::new()?, tokio::io::stdin(), tokio::io::stdout()).await
}
pub async fn serve() -> Result<()> {
    let path = config::socket_path();
    let parent = path.parent().context("invalidSocket")?;
    if !parent.exists() {
        config::private_dir(parent)?;
    }
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .mode(0o600)
        .open(path.with_extension("lock"))?;
    // The lock file is private to the current runtime directory and held for the process lifetime.
    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Ok(());
    }
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    let app = App::new()?;
    let probe = app.clone();
    tokio::spawn(async move {
        let available = models::engine_available().await;
        let ocr = crate::process::run("tesseract", &["--list-langs"], &[], 5, &CancellationToken::new())
            .await
            .unwrap_or_default();
        let mut s = probe.state.lock().await;
        s.snapshot.offline_engine = available;
        s.snapshot.ocr_installed = String::from_utf8_lossy(&ocr)
            .lines()
            .skip(1)
            .map(|l| l.trim().to_owned())
            .filter(|l| !l.is_empty() && l != "osd")
            .collect();
        probe.publish(&s);
        drop(s);
        probe.warm_models().await;
    });
    loop {
        tokio::select! {_=tokio::signal::ctrl_c()=>break,connection=listener.accept()=>{let (socket,_)=connection?;let app=app.clone();tokio::spawn(async move{let _=self::connection(app,socket).await;});}}
    }
    std::fs::remove_file(path)?;
    drop(lock);
    Ok(())
}
