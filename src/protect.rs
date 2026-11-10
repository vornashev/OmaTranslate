//! Keeps names out of machine translation: "Fable 5.2" must not become "Басня 5.2".
//!
//! Protected spans are swapped for word-like placeholders (`ZX0Q`) that Google,
//! MyMemory and LibreTranslate pass through unchanged, then swapped back.

/// What to keep untranslated in one segment.
pub struct Protected {
    pub text: String,
    originals: Vec<String>,
}

impl Protected {
    /// Nothing left to translate: the segment is only names, numbers and punctuation.
    pub fn is_untranslatable(&self) -> bool {
        let mut rest = self.text.clone();
        for i in 0..self.originals.len() {
            rest = rest.replace(&placeholder(i), "");
        }
        !rest.chars().any(char::is_alphabetic)
    }

    pub fn restore(&self, translated: &str) -> String {
        let mut out = translated.to_owned();
        for (i, original) in self.originals.iter().enumerate().rev() {
            out = replace_ignoring_case(&out, &placeholder(i), original);
        }
        out
    }

    pub fn original(&self) -> String {
        self.restore(&self.text)
    }
}

fn placeholder(i: usize) -> String {
    format!("ZX{i}Q")
}

fn replace_ignoring_case(haystack: &str, needle: &str, with: &str) -> String {
    let lower = haystack.to_ascii_lowercase();
    let needle = needle.to_ascii_lowercase();
    let mut out = String::with_capacity(haystack.len());
    let mut last = 0;
    for (at, _) in lower.match_indices(&needle) {
        out.push_str(&haystack[last..at]);
        out.push_str(with);
        last = at + needle.len();
    }
    out.push_str(&haystack[last..]);
    out
}

/// CamelCase inside a word (FrontierCode, OSWorld, iPhone): lowercase letters
/// with a capital past the first letter of a hyphen-free part.
fn camel(word: &str) -> bool {
    word.chars().count() >= 3
        && word.chars().any(char::is_lowercase)
        && word.split('-').any(|part| part.chars().skip(1).any(char::is_uppercase))
}

/// Letters and digits in one token (GPT-6, H100, v2, M4).
fn mixed(word: &str) -> bool {
    word.chars().any(|c| c.is_ascii_digit()) && word.chars().any(char::is_alphabetic)
}

/// A version after a name: 5.2, 2.0, 4, 0.1 — but not a year like 2026.
fn version(word: &str) -> bool {
    let digits = word.chars().filter(|c| c.is_ascii_digit()).count();
    word.starts_with(|c: char| c.is_ascii_digit())
        && word.chars().all(|c| c.is_ascii_digit() || c == '.')
        && (word.contains('.') || digits <= 2)
}

fn capitalized(word: &str) -> bool {
    word.starts_with(|c: char| c.is_uppercase())
}

/// Finds the spans to keep. `names` also keeps "Name 5.2"-style product names,
/// which is right for screenshots of tables and interfaces but too eager for prose.
pub fn protect(text: &str, glossary: &[String], names: bool) -> Protected {
    // Word spans with leading/trailing punctuation trimmed off.
    let mut words: Vec<(usize, usize)> = Vec::new();
    let mut start = None;
    for (i, c) in text.char_indices().chain(std::iter::once((text.len(), ' '))) {
        match (c.is_whitespace(), start) {
            (false, None) => start = Some(i),
            (true, Some(s)) => {
                let word = &text[s..i];
                let core = word.trim_start_matches(['(', '[', '«', '"', '\'']);
                let lead = word.len() - core.len();
                let core = core.trim_end_matches([',', '.', ';', ':', '!', '?', ')', ']', '»', '"', '\'']);
                if !core.is_empty() {
                    words.push((s + lead, s + lead + core.len()));
                }
                start = None;
            }
            _ => {}
        }
    }
    let token = |i: usize| &text[words[i].0..words[i].1];
    let mut keep = vec![false; words.len()];
    for i in 0..words.len() {
        let w = token(i);
        if camel(w) || mixed(w) {
            keep[i] = true;
        }
        let named_version = version(w) && i > 0 && capitalized(token(i - 1));
        // "GPT-6 Astra": a versioned name token can be followed by more name.
        let versioned_name = mixed(w) && capitalized(w);
        if names && (named_version || versioned_name) {
            keep[i] = true;
            if named_version {
                keep[i - 1] = true;
            }
            // "Gemini 4 Carbon": the name continues after the version.
            let mut j = i + 1;
            while j < words.len() && capitalized(token(j)) && !token(j).contains(['.', ',']) {
                keep[j] = true;
                j += 1;
            }
        }
    }
    // Byte ranges: rule hits plus glossary terms (whole words, case-sensitive).
    let mut ranges: Vec<(usize, usize)> = words.iter().zip(&keep).filter(|(_, k)| **k).map(|(r, _)| *r).collect();
    for term in glossary.iter().map(|t| t.trim()).filter(|t| !t.is_empty()) {
        for (at, _) in text.match_indices(term) {
            let end = at + term.len();
            let before = text[..at].chars().next_back().is_none_or(|c| !c.is_alphanumeric());
            let after = text[end..].chars().next().is_none_or(|c| !c.is_alphanumeric());
            if before && after {
                ranges.push((at, end));
            }
        }
    }
    ranges.sort();
    // Merge overlapping ranges and ones separated only by spaces.
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (s, e) in ranges {
        if let Some(last) = merged.last_mut()
            && (s <= last.1 || text[last.1..s].chars().all(|c| c == ' '))
        {
            last.1 = last.1.max(e);
            continue;
        }
        merged.push((s, e));
    }
    let mut out = String::with_capacity(text.len());
    let mut originals = Vec::new();
    let mut last = 0;
    for (s, e) in merged {
        out.push_str(&text[last..s]);
        out.push_str(&placeholder(originals.len()));
        originals.push(text[s..e].to_owned());
        last = e;
    }
    out.push_str(&text[last..]);
    Protected { text: out, originals }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_names_and_versions_are_kept_in_screens() {
        let p = protect("Fable 5.2", &[], true);
        assert!(p.is_untranslatable());
        assert_eq!(p.original(), "Fable 5.2");
        let p = protect("Gemini 4 Carbon", &[], true);
        assert_eq!(p.text, "ZX0Q");
        assert_eq!(protect("GPT-6 Astra", &[], true).text, "ZX0Q");
        let p = protect("Agentic coding FrontierCode 2.0", &[], true);
        assert_eq!(p.text, "Agentic coding ZX0Q");
        assert!(!p.is_untranslatable());
        assert_eq!(p.restore("Агентное кодирование zx0q"), "Агентное кодирование FrontierCode 2.0");
    }

    #[test]
    fn prose_keeps_years_and_section_numbers_translatable() {
        let p = protect("In 2026 we read Section 2 together.", &[], false);
        assert_eq!(p.text, "In 2026 we read Section 2 together.");
        let p = protect("Run GPT-6 on an H100, then open OSWorld.", &[], false);
        assert_eq!(p.text, "Run ZX0Q on an ZX1Q, then open ZX2Q.");
    }

    #[test]
    fn glossary_terms_match_whole_words_only() {
        let glossary = vec!["Omarchy".to_owned(), "Hyprland config".to_owned()];
        let p = protect("Omarchy ships a Hyprland config, not Omarchyish.", &glossary, false);
        assert_eq!(p.text, "ZX0Q ships a ZX1Q, not Omarchyish.");
        assert_eq!(p.restore("ZX0Q поставляет ZX1Q"), "Omarchy поставляет Hyprland config");
    }

    #[test]
    fn plain_text_is_untouched() {
        let p = protect("Small steps make a big difference.", &[], true);
        assert_eq!(p.text, "Small steps make a big difference.");
        assert_eq!(p.restore("Маленькие шаги"), "Маленькие шаги");
    }
}
