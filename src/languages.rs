//! Language catalog shared by detection, providers, OCR and the UI.
use serde_json::{Value, json};

/// (provider code, whatlang ISO 639-3, Tesseract code, endonym, English name)
pub const LANGUAGES: &[(&str, &str, &str, &str, &str)] = &[
    ("en", "eng", "eng", "English", "English"),
    ("ar", "ara", "ara", "العربية", "Arabic"),
    ("hy", "hye", "hye", "Հայերեն", "Armenian"),
    ("az", "aze", "aze", "Azərbaycanca", "Azerbaijani"),
    ("be", "bel", "bel", "Беларуская", "Belarusian"),
    ("bn", "ben", "ben", "বাংলা", "Bengali"),
    ("bg", "bul", "bul", "Български", "Bulgarian"),
    ("ca", "cat", "cat", "Català", "Catalan"),
    ("zh-CN", "cmn", "chi_sim", "简体中文", "Chinese (Simplified)"),
    ("zh-TW", "", "chi_tra", "繁體中文", "Chinese (Traditional)"),
    ("hr", "hrv", "hrv", "Hrvatski", "Croatian"),
    ("cs", "ces", "ces", "Čeština", "Czech"),
    ("da", "dan", "dan", "Dansk", "Danish"),
    ("nl", "nld", "nld", "Nederlands", "Dutch"),
    ("eo", "epo", "epo", "Esperanto", "Esperanto"),
    ("et", "est", "est", "Eesti", "Estonian"),
    ("fi", "fin", "fin", "Suomi", "Finnish"),
    ("fr", "fra", "fra", "Français", "French"),
    ("ka", "kat", "kat", "ქართული", "Georgian"),
    ("de", "deu", "deu", "Deutsch", "German"),
    ("el", "ell", "ell", "Ελληνικά", "Greek"),
    ("gu", "guj", "guj", "ગુજરાતી", "Gujarati"),
    ("he", "heb", "heb", "עברית", "Hebrew"),
    ("hi", "hin", "hin", "हिन्दी", "Hindi"),
    ("hu", "hun", "hun", "Magyar", "Hungarian"),
    ("id", "ind", "ind", "Bahasa Indonesia", "Indonesian"),
    ("it", "ita", "ita", "Italiano", "Italian"),
    ("ja", "jpn", "jpn", "日本語", "Japanese"),
    ("kk", "", "kaz", "Қазақ", "Kazakh"),
    ("ko", "kor", "kor", "한국어", "Korean"),
    ("lv", "lav", "lav", "Latviešu", "Latvian"),
    ("lt", "lit", "lit", "Lietuvių", "Lithuanian"),
    ("ms", "", "msa", "Bahasa Melayu", "Malay"),
    ("mr", "mar", "mar", "मराठी", "Marathi"),
    ("no", "nob", "nor", "Norsk", "Norwegian"),
    ("fa", "pes", "fas", "فارسی", "Persian"),
    ("pl", "pol", "pol", "Polski", "Polish"),
    ("pt", "por", "por", "Português", "Portuguese"),
    ("pa", "pan", "pan", "ਪੰਜਾਬੀ", "Punjabi"),
    ("ro", "ron", "ron", "Română", "Romanian"),
    ("ru", "rus", "rus", "Русский", "Russian"),
    ("sr", "srp", "srp", "Српски", "Serbian"),
    ("sk", "slk", "slk", "Slovenčina", "Slovak"),
    ("sl", "slv", "slv", "Slovenščina", "Slovenian"),
    ("es", "spa", "spa", "Español", "Spanish"),
    ("sv", "swe", "swe", "Svenska", "Swedish"),
    ("tl", "tgl", "tgl", "Tagalog", "Tagalog"),
    ("ta", "tam", "tam", "தமிழ்", "Tamil"),
    ("te", "tel", "tel", "తెలుగు", "Telugu"),
    ("th", "tha", "tha", "ไทย", "Thai"),
    ("tr", "tur", "tur", "Türkçe", "Turkish"),
    ("uk", "ukr", "ukr", "Українська", "Ukrainian"),
    ("ur", "urd", "urd", "اردو", "Urdu"),
    ("uz", "uzb", "uzb", "Oʻzbekcha", "Uzbek"),
    ("vi", "vie", "vie", "Tiếng Việt", "Vietnamese"),
];

/// Catalog for the UI: code, endonym, English name, Tesseract code.
pub fn catalog() -> Value {
    LANGUAGES
        .iter()
        .map(|(code, _, ocr, name, english)| {
            json!({"code": code, "name": name, "english": english, "ocr": ocr})
        })
        .collect()
}

/// Normalizes a locale (`de_DE.UTF-8`, `pt-BR`, `zh_TW`) to a catalog code.
pub fn from_locale(locale: &str) -> Option<&'static str> {
    let locale = locale.split(['.', '@']).next()?.replace('_', "-");
    let lower = locale.to_ascii_lowercase();
    if lower == "c" || lower == "posix" || lower.is_empty() {
        return None;
    }
    let base = lower.split('-').next()?;
    let code = match (base, lower.as_str()) {
        ("zh", "zh-tw" | "zh-hk" | "zh-mo" | "zh-hant") => "zh-TW",
        ("zh", _) => "zh-CN",
        ("nb" | "nn", _) => "no",
        ("iw", _) => "he",
        ("in", _) => "id",
        _ => base,
    };
    LANGUAGES.iter().find(|l| l.0 == code).map(|l| l.0)
}

/// Language of the desktop session, from the usual locale variables.
pub fn system() -> Option<&'static str> {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|v| std::env::var(v).ok())
        .find(|v| !v.is_empty())
        .and_then(|v| from_locale(&v))
}

/// Code spelling a provider expects for a catalog code.
pub fn provider_code<'a>(provider: &str, code: &'a str) -> &'a str {
    match (provider, code) {
        ("libre", "zh-CN") => "zh",
        ("libre", "zh-TW") => "zt",
        ("libre", "no") => "nb",
        _ => code,
    }
}

pub fn tesseract_code(code: &str) -> Option<&'static str> {
    LANGUAGES.iter().find(|l| l.0 == code).map(|l| l.2)
}

/// Detects the language of `text`, or `None` when the text has no letters.
/// `candidates` are the user's languages, most likely source first; they win
/// for short or ambiguous text.
#[cfg(test)]
pub fn detect(text: &str, candidates: &[&str]) -> Option<&'static str> {
    detect_reliably(text, candidates).map(|(code, _)| code)
}

/// Like `detect`, and whether the answer can be trusted: a script used by one
/// language, or a confident statistical match. Short text that was settled by
/// falling back to the user's own languages is not reliable.
pub fn detect_reliably(text: &str, candidates: &[&str]) -> Option<(&'static str, bool)> {
    let letters = text.chars().filter(|c| c.is_alphabetic()).count();
    if letters == 0 {
        return None;
    }
    let to_code = |lang: whatlang::Lang| {
        LANGUAGES
            .iter()
            .find(|l| !l.1.is_empty() && l.1 == lang.code())
            .map(|l| l.0)
    };
    // Scripts used by one language decide immediately, even for one word.
    if let Some(script) = whatlang::detect_script(text) {
        use whatlang::Script::*;
        let only = match script {
            Hangul => Some("ko"),
            Hiragana | Katakana => Some("ja"),
            Greek => Some("el"),
            Georgian => Some("ka"),
            Armenian => Some("hy"),
            Thai => Some("th"),
            Bengali => Some("bn"),
            Gujarati => Some("gu"),
            Gurmukhi => Some("pa"),
            Tamil => Some("ta"),
            Telugu => Some("te"),
            Hebrew => Some("he"),
            Cyrillic => return Some((cyrillic(text, candidates), true)),
            // Kana mixed with kanji is Japanese; plain Han is Chinese.
            Mandarin if text.chars().any(|c| matches!(c, '\u{3040}'..='\u{30ff}')) => Some("ja"),
            Mandarin => Some("zh-CN"),
            _ => None,
        };
        if let Some(code) = only {
            return Some((code, true));
        }
        // Short text in a shared script: choose among the user's languages
        // written in it (Cyrillic "Привет" is Russian for a Russian user).
        if letters < 24 {
            let same: Vec<_> = candidates
                .iter()
                .filter_map(|c| LANGUAGES.iter().find(|l| l.0 == *c))
                .filter_map(|l| whatlang::Lang::from_code(l.1).map(|lang| (l.0, lang)))
                .filter(|(_, lang)| script_of(*lang) == Some(script))
                .collect();
            match same.as_slice() {
                [] => {}
                [(code, _)] => return Some((code, false)),
                [(first, _), ..] => {
                    let allow = same.iter().map(|(_, lang)| *lang).collect();
                    return Some((
                        whatlang::Detector::with_allowlist(allow)
                            .detect(text)
                            .filter(whatlang::Info::is_reliable)
                            .and_then(|info| to_code(info.lang()))
                            .unwrap_or(first),
                        false,
                    ));
                }
            }
        }
    }
    let info = whatlang::detect(text)?;
    let detected = to_code(info.lang());
    if info.is_reliable() || candidates.is_empty() {
        return detected.map(|code| (code, info.is_reliable()));
    }
    // Unreliable: if one of the user's languages is close, take it.
    let allow: Vec<_> = candidates
        .iter()
        .filter_map(|c| LANGUAGES.iter().find(|l| l.0 == *c))
        .filter_map(|l| whatlang::Lang::from_code(l.1))
        .collect();
    if allow.is_empty() {
        return detected.map(|code| (code, false));
    }
    whatlang::Detector::with_allowlist(allow)
        .detect_lang(text)
        .and_then(to_code)
        .or(detected)
        .map(|code| (code, false))
}

/// whatlang often mistakes Russian for Bulgarian, so Cyrillic is decided by
/// letters unique to each language; unmarked text is the user's Cyrillic
/// language, or Russian, the most widely used one.
fn cyrillic(text: &str, candidates: &[&str]) -> &'static str {
    let has = |set: &str| text.chars().any(|c| set.contains(c.to_lowercase().next().unwrap_or(c)));
    if has("ў") {
        return "be";
    }
    if has("їєґ") || (has("і") && !has("ыэё")) {
        return "uk";
    }
    if has("ђћџљњј") {
        return "sr";
    }
    if has("ыэё") {
        return "ru";
    }
    for code in candidates {
        if let Some(found) = ["ru", "uk", "be", "bg", "sr", "kk"].into_iter().find(|c| c == code) {
            return found;
        }
    }
    "ru"
}

fn script_of(lang: whatlang::Lang) -> Option<whatlang::Script> {
    use whatlang::{Lang::*, Script};
    Some(match lang {
        Rus | Ukr | Bel | Bul | Srp | Mkd => Script::Cyrillic,
        Ara | Pes | Urd => Script::Arabic,
        Hin | Mar | Nep => Script::Devanagari,
        Cmn => Script::Mandarin,
        Jpn => return None,
        _ => Script::Latin,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn locales_map_to_catalog_codes() {
        assert_eq!(from_locale("de_DE.UTF-8"), Some("de"));
        assert_eq!(from_locale("pt_BR.UTF-8"), Some("pt"));
        assert_eq!(from_locale("zh_TW.UTF-8"), Some("zh-TW"));
        assert_eq!(from_locale("zh_CN"), Some("zh-CN"));
        assert_eq!(from_locale("nb_NO.UTF-8"), Some("no"));
        assert_eq!(from_locale("C.UTF-8"), None);
        assert_eq!(from_locale("POSIX"), None);
    }
    #[test]
    fn detects_common_languages() {
        let cases = [
            ("The quick brown fox jumps over the lazy dog every morning", "en"),
            ("Привет, как у тебя сегодня дела на работе?", "ru"),
            ("Der schnelle braune Fuchs springt über den faulen Hund", "de"),
            ("Le renard brun rapide saute par-dessus le chien paresseux", "fr"),
            ("El rápido zorro marrón salta sobre el perro perezoso", "es"),
            ("Привіт, як у тебе сьогодні справи на роботі? Їжак", "uk"),
            ("こんにちは、元気ですか", "ja"),
            ("今天天气很好", "zh-CN"),
            ("안녕하세요", "ko"),
            ("Καλημέρα σας", "el"),
            ("שלום עולם", "he"),
        ];
        for (text, code) in cases {
            assert_eq!(detect(text, &[]), Some(code), "{text}");
        }
    }
    #[test]
    fn short_text_prefers_user_languages() {
        assert_eq!(detect("Привет", &["ru", "en"]), Some("ru"));
        assert_eq!(detect("Hello", &["en", "de"]), Some("en"));
        assert_eq!(detect("Привет", &["en", "uk"]), Some("uk"));
        assert_eq!(detect("Добрий день, їжак", &["en", "ru"]), Some("uk"));
        assert_eq!(detect("12345", &["ru"]), None);
        // Only confident answers are reliable; short Latin text falls back to the user's languages.
        assert_eq!(detect_reliably("Das Wetter ist heute sehr schön", &["en", "ru"]), Some(("de", true)));
        assert_eq!(detect_reliably("Привет", &["en", "ru"]), Some(("ru", true)));
        assert_eq!(detect_reliably("こんにちは", &["en"]), Some(("ja", true)));
        assert_eq!(detect_reliably("Guten Morgen, wie geht es dir heute?", &["en", "ru"]).map(|d| d.1), Some(false));
    }
}
