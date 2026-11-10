use crate::{config, process};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
};
use tokio_util::sync::CancellationToken;

pub const PAIRS: [&str; 2] = ["en_ru", "ru_en"];
pub fn model_dir(pair: &str) -> PathBuf {
    config::data_dir().join("models").join(pair)
}
pub fn status() -> Vec<Value> {
    PAIRS.iter().map(|p|json!({"pair":p,"installed":model_dir(p).join("integrity.json").is_file(),"version":"1.9"})).collect()
}
pub fn worker_path() -> PathBuf {
    if let Some(p) = std::env::var_os("OMATRANSLATE_WORKER") {
        return p.into();
    }
    config::resource_path("worker/offline.py", "offline.py")
}
pub fn python() -> PathBuf {
    if let Some(p) = std::env::var_os("OMATRANSLATE_PYTHON") {
        return p.into();
    }
    let local = config::data_dir().join("venv/bin/python");
    if local.exists() {
        local
    } else {
        PathBuf::from("python3")
    }
}
pub struct Offline {
    child: Option<Child>,
    output: Option<BufReader<tokio::process::ChildStdout>>,
}
impl Offline {
    pub fn new() -> Self {
        Self {
            child: None,
            output: None,
        }
    }
    async fn start(&mut self) -> Result<()> {
        if self.child.is_some() {
            return Ok(());
        }
        let mut child = Command::new(python())
            .arg(worker_path())
            .arg(config::data_dir().join("models"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .context("offlineUnavailable")?;
        let mut output = BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        tokio::time::timeout(Duration::from_secs(10), output.read_line(&mut line))
            .await
            .context("offlineUnavailable")??;
        if !line.contains("ready") {
            bail!("offlineUnavailable");
        }
        self.child = Some(child);
        self.output = Some(output);
        Ok(())
    }
    pub fn reset(&mut self) {
        self.child.take();
        self.output.take();
    }
    pub async fn translate(
        &mut self,
        segments: &[String],
        source: &str,
        target: &str,
        warm: bool,
        cancel: &CancellationToken,
    ) -> Result<Vec<String>> {
        if cancel.is_cancelled() {
            bail!("cancelled");
        }
        self.start().await?;
        let request =
            json!({"segments":segments,"source":source,"target":target,"keepModelsWarm":warm});
        self.child
            .as_mut()
            .unwrap()
            .stdin
            .as_mut()
            .unwrap()
            .write_all(format!("{request}\n").as_bytes())
            .await
            .context("offlineUnavailable")?;
        let mut line = String::new();
        let read = self.output.as_mut().unwrap().read_line(&mut line);
        let result = tokio::select! {_=cancel.cancelled()=>Err(anyhow::anyhow!("cancelled")),r=tokio::time::timeout(Duration::from_secs(90),read)=>r.context("timeout")?.context("offlineUnavailable")};
        if result.is_err() || line.len() > 4 * 1024 * 1024 {
            self.reset();
            result?;
            bail!("invalidResponse");
        }
        let value: Value = serde_json::from_str(&line).context("offlineUnavailable")?;
        if let Some(code) = value["error"].as_str() {
            bail!("{}", code);
        }
        let output: Vec<String> =
            serde_json::from_value(value["translations"].clone()).context("invalidResponse")?;
        if output.len() != segments.len() {
            bail!("invalidResponse");
        }
        Ok(output)
    }
}
pub async fn engine_available() -> bool {
    process::run(
        python().to_str().unwrap_or("python3"),
        &["-c", "import ctranslate2, sentencepiece"],
        &[],
        10,
        &CancellationToken::new(),
    )
    .await
    .is_ok()
}
pub async fn download(
    pair: &str,
    cancel: &CancellationToken,
    progress: impl Fn(u64, Option<u64>),
) -> Result<()> {
    if !PAIRS.contains(&pair) {
        bail!("invalidModel");
    }
    let root = config::data_dir().join("models");
    config::private_dir(&root)?;
    let mut archive = tempfile::NamedTempFile::new_in(&root)?;
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .read_timeout(Duration::from_secs(30))
        .https_only(true)
        .build()?;
    let url = format!("https://argos-net.com/v1/translate-{pair}-1_9.argosmodel");
    let work = async {
        let mut response = client
            .get(&url)
            .send()
            .await
            .context("downloadFailed")?
            .error_for_status()
            .context("downloadFailed")?;
        let total = response.content_length();
        let mut received = 0;
        while let Some(bytes) = response.chunk().await.context("downloadFailed")? {
            received += bytes.len() as u64;
            if received > 1024 * 1024 * 1024 {
                bail!("invalidModel");
            }
            archive.write_all(&bytes).context("writeFailed")?;
            progress(received, total);
        }
        archive.flush()?;
        // The upstream index has no published digests. A release lock, when provided,
        // pins the archive; installed-file digests always detect later corruption.
        let lock_path = std::env::var_os("OMATRANSLATE_MODEL_LOCK")
            .map(PathBuf::from)
            .unwrap_or_else(|| config::resource_path("models.lock.json", "models.lock.json"));
        if let Ok(bytes) = std::fs::read(lock_path) {
            let lock: Value = serde_json::from_slice(&bytes)?;
            if let Some(expected) = lock[pair]["sha256"].as_str()
                && digest_file(archive.path())? != expected
            {
                bail!("invalidModel");
            }
        }
        Ok::<_, anyhow::Error>(())
    };
    tokio::select! {_=cancel.cancelled()=>bail!("cancelled"),r=work=>r?};
    let pair = pair.to_owned();
    tokio::task::spawn_blocking(move || unpack(archive.path(), &pair, &url)).await??;
    Ok(())
}
pub fn digest_file(path: &Path) -> Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut h = Sha256::new();
    let mut b = [0u8; 65536];
    loop {
        let n = f.read(&mut b)?;
        if n == 0 {
            break;
        }
        h.update(&b[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}
pub fn unpack(path: &Path, pair: &str, url: &str) -> Result<()> {
    let root = config::data_dir().join("models");
    unpack_into(path, pair, url, &root)
}
fn unpack_into(path: &Path, pair: &str, url: &str, root: &Path) -> Result<()> {
    let staging = tempfile::tempdir_in(root)?;
    let mut zip = zip::ZipArchive::new(std::fs::File::open(path)?).context("invalidModel")?;
    if zip.len() > 5000 {
        bail!("invalidModel");
    }
    let mut total = 0u64;
    let mut prefix = None;
    let mut hashes = BTreeMap::new();
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).context("invalidModel")?;
        let safe = entry.enclosed_name().context("invalidModel")?;
        if entry.is_symlink() || entry.name().contains('\\') {
            bail!("invalidModel");
        }
        let mut components = safe.components();
        let first = components
            .next()
            .context("invalidModel")?
            .as_os_str()
            .to_owned();
        if prefix.as_ref().is_some_and(|p| p != &first) {
            bail!("invalidModel");
        }
        prefix = Some(first);
        let relative = components.as_path();
        if relative.as_os_str().is_empty() {
            continue;
        }
        let dest = staging.path().join(relative);
        if entry.is_dir() {
            std::fs::create_dir_all(&dest)?;
            continue;
        }
        total = total.checked_add(entry.size()).context("invalidModel")?;
        if total > 2 * 1024 * 1024 * 1024 {
            bail!("invalidModel");
        }
        std::fs::create_dir_all(dest.parent().unwrap())?;
        let mut out = std::fs::File::create(&dest)?;
        let actual = std::io::copy(
            &mut entry.by_ref().take(2 * 1024 * 1024 * 1024 + 1),
            &mut out,
        )?;
        if actual != entry.size() {
            bail!("invalidModel");
        }
        hashes.insert(relative.to_string_lossy().into_owned(), digest_file(&dest)?);
    }
    if !staging.path().join("model/model.bin").is_file()
        || !staging.path().join("sentencepiece.model").is_file()
    {
        bail!("invalidModel");
    }
    config::atomic_write(
        &staging.path().join("integrity.json"),
        &serde_json::to_vec_pretty(
            &json!({"files":hashes,"archiveSha256":digest_file(path)?,"version":"1.9","source":url}),
        )?,
    )?;
    let dest = root.join(pair);
    let backup = root.join(format!("{pair}.previous"));
    if backup.exists() {
        std::fs::remove_dir_all(&backup)?;
    }
    if dest.exists() {
        std::fs::rename(&dest, &backup)?;
    }
    if let Err(error) = std::fs::rename(staging.path(), &dest) {
        if backup.exists() {
            let _ = std::fs::rename(&backup, &dest);
        }
        return Err(error.into());
    }
    if backup.exists() {
        std::fs::remove_dir_all(&backup)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn archive(path: &Path, entries: &[(&str, &[u8])]) {
        let mut writer = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        for (name, bytes) in entries {
            writer
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap();
    }
    #[test]
    fn archive_escape_is_rejected() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("bad.zip");
        archive(&path, &[("package/../../escape", b"bad")]);
        assert!(unpack_into(&path, "en_ru", "test", root.path()).is_err());
        assert!(!root.path().join("en_ru").exists());
    }
    #[test]
    fn model_install_records_hashes_and_replaces_atomically() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("model.zip");
        archive(
            &path,
            &[
                ("package/model/model.bin", b"weights"),
                ("package/sentencepiece.model", b"tokenizer"),
            ],
        );
        unpack_into(&path, "en_ru", "test", root.path()).unwrap();
        let data: Value = serde_json::from_slice(
            &std::fs::read(root.path().join("en_ru/integrity.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            data["files"]["model/model.bin"],
            format!("{:x}", Sha256::digest(b"weights"))
        );
        archive(
            &path,
            &[
                ("package/model/model.bin", b"new"),
                ("package/sentencepiece.model", b"tokenizer"),
            ],
        );
        unpack_into(&path, "en_ru", "test", root.path()).unwrap();
        assert_eq!(
            std::fs::read(root.path().join("en_ru/model/model.bin")).unwrap(),
            b"new"
        );
        assert!(!root.path().join("en_ru.previous").exists());
    }
}
