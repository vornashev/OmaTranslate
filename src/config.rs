use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub provider: String,
    pub source: String,
    pub target: String,
    /// When auto-detected text is already in `target`, translate it into
    /// `second_language` instead (formerly Russian-only `reverseRussian`).
    #[serde(alias = "reverseRussian")]
    pub auto_swap: bool,
    pub second_language: String,
    pub auto_translate: bool,
    pub debounce_ms: u64,
    pub libre_url: String,
    pub libre_key: String,
    pub ocr_languages: String,
    pub min_font_size: u32,
    pub keep_models_warm: bool,
    pub history: bool,
    pub selection_key: String,
    pub clipboard_key: String,
    pub screenshot_key: String,
    pub replace_key: String,
    /// Words and names never sent for translation (product names, terms).
    pub glossary: Vec<String>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            provider: "google".into(),
            source: "auto".into(),
            target: crate::languages::system().unwrap_or("en").into(),
            auto_swap: true,
            second_language: "en".into(),
            auto_translate: true,
            debounce_ms: 250,
            libre_url: "http://localhost:5000".into(),
            libre_key: String::new(),
            ocr_languages: default_ocr_languages(),
            min_font_size: 12,
            keep_models_warm: false,
            history: false,
            selection_key: "SUPER + U".into(),
            clipboard_key: "SUPER + SHIFT + U".into(),
            screenshot_key: "SUPER + ALT + U".into(),
            replace_key: "SUPER + CTRL + U".into(),
            glossary: Vec::new(),
        }
    }
}
impl Settings {
    pub fn validate(&mut self) -> Result<()> {
        if !["google", "mymemory", "libre", "local"].contains(&self.provider.as_str()) {
            bail!("invalidProvider");
        }
        for code in [&self.source, &self.target, &self.second_language] {
            if code.is_empty()
                || code.len() > 12
                || !code.bytes().all(|b| b.is_ascii_alphabetic() || b == b'-')
            {
                bail!("invalidLanguage");
            }
        }
        if self.target == "auto" || self.second_language == "auto" {
            bail!("invalidLanguage");
        }
        if self.ocr_languages.len() > 100
            || !self
                .ocr_languages
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'_')
        {
            bail!("invalidLanguage");
        }
        self.debounce_ms = self.debounce_ms.clamp(100, 2000);
        self.glossary = self
            .glossary
            .iter()
            .map(|t| t.trim().to_owned())
            .filter(|t| !t.is_empty())
            .collect();
        if self.glossary.len() > 200 || self.glossary.iter().any(|t| t.chars().count() > 100) {
            bail!("invalidGlossary");
        }
        self.min_font_size = self.min_font_size.clamp(10, 28);
        for key in [
            &self.selection_key,
            &self.clipboard_key,
            &self.screenshot_key,
            &self.replace_key,
        ] {
            if key.len() > 100
                || !key
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b' ' || b == b'+' || b == b':')
            {
                bail!("invalidShortcut");
            }
        }
        Ok(())
    }
    pub fn load() -> Self {
        let loaded = fs::read(config_dir().join("settings.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<Self>(&b).ok());
        if let Some(mut s) = loaded
            && s.validate().is_ok()
        {
            return s;
        }
        Self::default()
    }
    pub fn save(&self) -> Result<()> {
        atomic_write(
            &config_dir().join("settings.json"),
            &serde_json::to_vec_pretty(self)?,
        )
    }
    pub fn languages(&self, text: &str) -> (String, String) {
        let source = if self.source == "auto" {
            crate::languages::detect(text, &[&self.second_language, &self.target])
                .unwrap_or(if self.target == self.second_language { "en" } else { &self.second_language })
                .to_owned()
        } else {
            self.source.clone()
        };
        let target = if self.auto_swap
            && self.source == "auto"
            && source == self.target
            && self.second_language != self.target
        {
            self.second_language.clone()
        } else {
            self.target.clone()
        };
        (source, target)
    }
}
/// English plus the session language, e.g. `eng+deu` under `de_DE.UTF-8`.
fn default_ocr_languages() -> String {
    match crate::languages::system().and_then(crate::languages::tesseract_code) {
        Some(code) if code != "eng" => format!("eng+{code}"),
        _ => "eng".into(),
    }
}
pub fn home() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
}
pub fn config_dir() -> PathBuf {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".config"))
        .join("omatranslate")
}
pub fn data_dir() -> PathBuf {
    env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".local/share"))
        .join("omatranslate")
}
pub fn resource_path(development: &str, installed: &str) -> PathBuf {
    #[cfg(not(feature = "packaged"))]
    {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(development);
        if path.exists() {
            return path;
        }
    }
    #[cfg(feature = "packaged")]
    let _ = development;
    PathBuf::from("/usr/share/omatranslate").join(installed)
}
pub fn socket_path() -> PathBuf {
    if let Some(p) = env::var_os("OMATRANSLATE_SOCKET") {
        return p.into();
    }
    let root = env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(format!("/tmp/omatranslate-{}", unsafe { libc::geteuid() }))
        });
    let display: String = env::var("WAYLAND_DISPLAY")
        .unwrap_or_else(|_| "default".into())
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .take(32)
        .collect();
    root.join(format!("omatranslate-{display}.sock"))
}
pub fn private_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().context("writeFailed")?;
    fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent).context("writeFailed")?;
    file.write_all(bytes).context("writeFailed")?;
    file.as_file().sync_all()?;
    file.persist(path)
        .map_err(|_| anyhow::anyhow!("writeFailed"))?;
    Ok(())
}
pub fn split_text(text: &str, max: usize) -> Vec<String> {
    let mut remaining = text;
    let mut result = Vec::new();
    while !remaining.is_empty() {
        let mut end = remaining.len().min(max);
        while !remaining.is_char_boundary(end) {
            end -= 1;
        }
        if end == 0 {
            end = remaining.chars().next().unwrap().len_utf8();
        }
        if end < remaining.len()
            && let Some((i, c)) = remaining[..end]
                .char_indices()
                .rev()
                .find(|(i, c)| *i > end / 2 && c.is_whitespace())
        {
            end = i + c.len_utf8();
        }
        result.push(remaining[..end].to_owned());
        remaining = &remaining[end..];
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn splitting_preserves_unicode_and_whitespace() {
        let text = "Привет, мир 🌍!\n".repeat(100);
        let parts = split_text(&text, 500);
        assert!(parts.iter().all(|p| p.len() <= 500));
        assert_eq!(parts.concat(), text);
    }
    #[test]
    fn auto_swap_uses_second_language() {
        let mut s = Settings { target: "ru".into(), ..Settings::default() };
        assert_eq!(s.languages("Привет"), ("ru".into(), "en".into()));
        assert_eq!(s.languages("Hello there").1, "ru");
        s.auto_swap = false;
        assert_eq!(s.languages("Привет").1, "ru");
        let s = Settings { target: "de".into(), second_language: "en".into(), ..Settings::default() };
        assert_eq!(s.languages("Guten Morgen, wie geht es dir heute?"), ("de".into(), "en".into()));
        assert_eq!(s.languages("Good morning, how are you today?"), ("en".into(), "de".into()));
    }
    #[test]
    fn legacy_reverse_russian_is_migrated() {
        let s: Settings = serde_json::from_str(r#"{"target":"ru","reverseRussian":false}"#).unwrap();
        assert!(!s.auto_swap);
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("autoSwap") && !json.contains("reverseRussian"));
    }
}
