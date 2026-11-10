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
    /// The user's own language: foreign text is translated into it.
    pub target: String,
    /// When the text is already in `target`, translate it into the second
    /// language instead (formerly Russian-only `reverseRussian`).
    #[serde(alias = "reverseRussian")]
    pub auto_swap: bool,
    /// `"last"` follows `last_language`; a language code pins the second language.
    pub second_language: String,
    /// The foreign language the user worked with most recently: the language of
    /// incoming foreign text, or a target picked by hand.
    pub last_language: String,
    /// Settings format. Files written before the "last language" mode read as 0.
    #[serde(default)]
    pub settings_version: u32,
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
            second_language: "last".into(),
            last_language: "en".into(),
            settings_version: SETTINGS_VERSION,
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
        for code in [&self.source, &self.target, &self.second_language, &self.last_language] {
            if code.is_empty()
                || code.len() > 12
                || !code.bytes().all(|b| b.is_ascii_alphabetic() || b == b'-')
            {
                bail!("invalidLanguage");
            }
        }
        if self.target == "auto"
            || self.second_language == "auto"
            || ["auto", "last"].contains(&self.last_language.as_str())
            || ["last"].contains(&self.source.as_str())
            || self.target == "last"
        {
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
            if s.migrate() {
                let _ = s.save();
            }
            return s;
        }
        Self::default()
    }
    /// Moves older settings to the "my language + last language" mode once.
    ///
    /// A fixed source language meant a manual pair, often left behind by the old
    /// swap button, which switched automatic mode off for good. The target ("translate
    /// into") becomes the user's own language and the source the last one. Neither the
    /// system language (many people run an English desktop) nor the swap state (the
    /// button toggled back and forth) tells reliably which side is theirs; the target
    /// is what they last chose to read, and "My language" is one click to change.
    pub fn migrate(&mut self) -> bool {
        if self.settings_version >= SETTINGS_VERSION {
            return false;
        }
        if self.source != "auto" {
            self.last_language = std::mem::replace(&mut self.source, "auto".into());
        } else if self.second_language != "last" {
            self.last_language = self.second_language.clone();
        }
        self.second_language = "last".into();
        self.auto_swap = true;
        self.settings_version = SETTINGS_VERSION;
        true
    }
    /// The second language in effect: the pinned one, or the last one used.
    pub fn second(&self) -> &str {
        if self.second_language == "last" {
            &self.last_language
        } else {
            &self.second_language
        }
    }
    pub fn save(&self) -> Result<()> {
        atomic_write(
            &config_dir().join("settings.json"),
            &serde_json::to_vec_pretty(self)?,
        )
    }
    /// Where a text goes. `source` and `target` override the settings for one
    /// request (a language picked in the panel, the swap button).
    pub fn direction(&self, text: &str, source: Option<&str>, target: Option<&str>) -> Direction {
        let second = self.second();
        let detected = source.is_none() && self.source == "auto";
        let (source, reliable) = match source {
            Some(code) => (code.to_owned(), true),
            None if detected => crate::languages::detect_reliably(text, &[second, &self.target])
                .map(|(code, reliable)| (code.to_owned(), reliable))
                .unwrap_or_else(|| ((if self.target == second { "en" } else { second }).to_owned(), false)),
            None => (self.source.clone(), true),
        };
        let (target, reason) = if let Some(code) = target {
            (code.to_owned(), "manual")
        } else if self.auto_swap && source == self.target && second != self.target {
            let reason = if self.second_language == "last" { "last" } else { "second" };
            (second.to_owned(), reason)
        } else {
            (self.target.clone(), "native")
        };
        Direction { source, target, reason, detected, reliable }
    }
    /// Updates the last language from a finished direction. Returns whether it changed.
    ///
    /// A target picked by hand always counts. A detected foreign source counts
    /// only for text long enough to be detected reliably.
    pub fn learn(&mut self, text: &str, direction: &Direction) -> bool {
        let next = if direction.reason == "manual" && direction.target != self.target {
            Some(&direction.target)
        } else if direction.detected
            && direction.reliable
            && direction.source != self.target
            && reliable(text)
        {
            Some(&direction.source)
        } else {
            None
        };
        match next {
            Some(code) if *code != self.last_language => {
                self.last_language = code.clone();
                true
            }
            _ => false,
        }
    }
}

/// The settings format written by this version.
pub const SETTINGS_VERSION: u32 = 2;

#[derive(Clone, Debug, PartialEq)]
pub struct Direction {
    pub source: String,
    pub target: String,
    /// "native", "last" (own language → last language), "second" (→ pinned) or "manual".
    pub reason: &'static str,
    /// The source language was detected rather than given.
    pub detected: bool,
    /// The source is given, or detection was confident. An unreliable guess is
    /// still used to choose the target, but services that detect better get "auto".
    pub reliable: bool,
}

/// Enough text to trust language detection: about two words. Ideographs carry
/// more per character than letters do.
fn reliable(text: &str) -> bool {
    let weight: usize = text
        .chars()
        .map(|c| match c as u32 {
            0x3040..=0x30FF | 0x3400..=0x9FFF | 0xAC00..=0xD7AF => 3,
            _ if c.is_alphabetic() => 1,
            _ => 0,
        })
        .sum();
    weight >= 10
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
    fn pair(d: Direction) -> (String, String) {
        (d.source, d.target)
    }
    #[test]
    fn own_language_goes_to_the_last_language() {
        let mut s = Settings { target: "ru".into(), ..Settings::default() };
        assert_eq!(pair(s.direction("Привет", None, None)), ("ru".into(), "en".into()));
        assert_eq!(s.direction("Привет", None, None).reason, "last");
        assert_eq!(s.direction("Hello there", None, None).target, "ru");
        s.auto_swap = false;
        assert_eq!(s.direction("Привет", None, None).target, "ru");
        let s = Settings { target: "de".into(), second_language: "en".into(), ..Settings::default() };
        let d = s.direction("Guten Morgen, wie geht es dir heute?", None, None);
        assert_eq!((pair(d.clone()), d.reason), (("de".into(), "en".into()), "second"));
        assert_eq!(pair(s.direction("Good morning, how are you today?", None, None)), ("en".into(), "de".into()));
    }
    #[test]
    fn foreign_text_and_picked_targets_update_the_last_language() {
        let mut s = Settings { target: "ru".into(), ..Settings::default() };
        let text = "Das Wetter ist heute sehr schön";
        let d = s.direction(text, None, None);
        assert_eq!(pair(d.clone()), ("de".into(), "ru".into()));
        assert!(s.learn(text, &d));
        assert_eq!(s.last_language, "de");
        // Now Russian text goes to German.
        assert_eq!(s.direction("Доброе утро, как дела сегодня?", None, None).target, "de");
        // A word or two doesn't move it: detection is unreliable there.
        let d = s.direction("Hello", None, None);
        assert!(!s.learn("Hello", &d));
        assert_eq!(s.last_language, "de");
        // A target picked by hand does, even for short text.
        let d = s.direction("Привет", None, Some("es"));
        assert_eq!((d.target.as_str(), d.reason), ("es", "manual"));
        assert!(s.learn("Привет", &d));
        assert_eq!(s.last_language, "es");
        // Picking the own language by hand or translating own-language text changes nothing.
        let d = s.direction("Hola amigos, buenos días a todos", None, Some("ru"));
        assert!(!s.learn("Hola amigos, buenos días a todos", &d));
        // An unreliable guess for short foreign text doesn't move it either.
        let d = s.direction("Guten Morgen, wie geht es dir heute?", None, None);
        assert!(!d.reliable);
        assert!(!s.learn("Guten Morgen, wie geht es dir heute?", &d));
        let d = s.direction("Доброе утро, как дела сегодня?", None, None);
        assert!(!s.learn("Доброе утро, как дела сегодня?", &d));
        assert_eq!(s.last_language, "es");
        // The swap button: both sides given for one request.
        let d = s.direction("Hola", Some("ru"), Some("es"));
        assert_eq!((pair(d.clone()), d.detected), (("ru".into(), "es".into()), false));
    }
    #[test]
    fn pinned_second_language_ignores_the_last_one() {
        let s = Settings { target: "ru".into(), second_language: "en".into(), last_language: "de".into(), ..Settings::default() };
        assert_eq!(s.direction("Доброе утро, как дела сегодня?", None, None).target, "en");
    }
    #[test]
    fn manual_pairs_become_own_and_last_language_once() {
        // The old swap button left a fixed pair behind, with automatic mode off.
        let mut s: Settings = serde_json::from_str(
            r#"{"source":"en","target":"ru","autoSwap":false,"secondLanguage":"ru"}"#,
        )
        .unwrap();
        assert_eq!(s.settings_version, 0);
        assert!(s.migrate());
        assert_eq!((s.source.as_str(), s.target.as_str(), s.last_language.as_str()), ("auto", "ru", "en"));
        assert_eq!((s.second_language.as_str(), s.auto_swap), ("last", true));
        assert!(!s.migrate(), "migrates only once");
        // Automatic mode with a pinned second language keeps it as the last one.
        let mut s: Settings = serde_json::from_str(r#"{"source":"auto","target":"de","secondLanguage":"fr"}"#).unwrap();
        s.migrate();
        assert_eq!((s.second_language.as_str(), s.last_language.as_str()), ("last", "fr"));
        assert!(Settings::default().settings_version == SETTINGS_VERSION);
    }
    #[test]
    fn legacy_reverse_russian_is_migrated() {
        let s: Settings = serde_json::from_str(r#"{"target":"ru","reverseRussian":false}"#).unwrap();
        assert!(!s.auto_swap);
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("autoSwap") && !json.contains("reverseRussian"));
    }
}
