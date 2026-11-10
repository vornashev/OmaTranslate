use crate::config::{Settings, split_text};
use anyhow::{Context, Result, bail};
use futures_util::{StreamExt, TryStreamExt, stream};
use reqwest::{Client, Url};
use serde_json::{Value, json};
use std::{collections::VecDeque, sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

type CacheKey = (String, String, String, String, String);
pub struct Providers {
    client: Client,
    cache: Mutex<VecDeque<(CacheKey, String)>>,
    pub google: Url,
    pub memory: Url,
}
impl Providers {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .connect_timeout(Duration::from_secs(4))
                .redirect(reqwest::redirect::Policy::none())
                .user_agent("OmaTranslate/0.1")
                .build()?,
            cache: Mutex::new(VecDeque::new()),
            google: Url::parse("https://translate.googleapis.com/translate_a/single")?,
            memory: Url::parse("https://api.mymemory.translated.net/get")?,
        })
    }
    pub async fn translate(
        self: &Arc<Self>,
        segments: &[String],
        s: &Settings,
        source: &str,
        target: &str,
        cancel: &CancellationToken,
    ) -> Result<Vec<String>> {
        if source == target {
            return Ok(segments.to_vec());
        }
        let mut parts = Vec::new();
        for (index, text) in segments.iter().enumerate() {
            // Newlines are layout, not sentence punctuation; preserve them independently of the provider.
            for line in text.split_inclusive('\n') {
                let content = line.strip_suffix('\n').unwrap_or(line);
                for part in split_text(content, if s.provider == "mymemory" { 500 } else { 4000 }) {
                    parts.push((index, part));
                }
                if line.ends_with('\n') {
                    parts.push((index, "\n".into()));
                }
            }
        }
        let results: Vec<(usize, String)> = stream::iter(parts)
            .map(|(i, text)| async move {
                Ok::<_, anyhow::Error>((i, self.one(&text, s, source, target, cancel).await?))
            })
            .buffered(3)
            .try_collect()
            .await?;
        let mut result = vec![String::new(); segments.len()];
        for (i, text) in results {
            result[i].push_str(&text);
        }
        Ok(result)
    }
    async fn one(
        &self,
        text: &str,
        s: &Settings,
        source: &str,
        target: &str,
        cancel: &CancellationToken,
    ) -> Result<String> {
        if text.trim().is_empty() {
            return Ok(text.into());
        }
        let key = (
            s.provider.clone(),
            format!("{}\0{}", s.libre_url, s.libre_key),
            source.into(),
            target.into(),
            text.into(),
        );
        if let Some((_, value)) = self.cache.lock().await.iter().find(|(k, _)| k == &key) {
            return Ok(value.clone());
        }
        let source = crate::languages::provider_code(&s.provider, source);
        let target = crate::languages::provider_code(&s.provider, target);
        let request = match s.provider.as_str() {
            "google" => {
                let mut url = self.google.clone();
                url.query_pairs_mut().extend_pairs([
                    ("client", "gtx"),
                    ("sl", source),
                    ("tl", target),
                    ("dt", "t"),
                    ("q", text),
                ]);
                self.client.get(url)
            }
            "mymemory" => {
                let mut url = self.memory.clone();
                url.query_pairs_mut()
                    .extend_pairs([("q", text), ("langpair", &format!("{source}|{target}"))]);
                self.client.get(url)
            }
            "libre" => {
                let mut url = Url::parse(&s.libre_url).context("invalidUrl")?;
                if !url.username().is_empty()
                    || url.password().is_some()
                    || !(url.scheme() == "https"
                        || (url.scheme() == "http"
                            && ["localhost", "127.0.0.1", "[::1]"]
                                .contains(&url.host_str().unwrap_or(""))))
                {
                    bail!("invalidUrl");
                }
                url.set_path(&format!("{}/translate", url.path().trim_end_matches('/')));
                url.set_query(None);
                url.set_fragment(None);
                self.client.post(url).json(&json!({"q":text,"source":source,"target":target,"format":"text","api_key":s.libre_key}))
            }
            _ => bail!("invalidProvider"),
        };
        let work = async {
            let mut response = request.send().await.context("networkError")?;
            if [403, 429].contains(&response.status().as_u16()) {
                bail!("rateLimited");
            }
            if !response.status().is_success() {
                bail!("providerError");
            }
            let mut body = Vec::new();
            while let Some(chunk) = response.chunk().await.context("networkError")? {
                if body.len() + chunk.len() > 1024 * 1024 {
                    bail!("invalidResponse");
                }
                body.extend_from_slice(&chunk);
            }
            parse(&s.provider, &body)
        };
        let mut result = tokio::select! { _=cancel.cancelled()=>bail!("cancelled"), r=work=>r? };
        if text.ends_with(' ') && !result.ends_with(' ') {
            result.push(' ');
        }
        let mut cache = self.cache.lock().await;
        cache.push_front((key, result.clone()));
        cache.truncate(512);
        Ok(result)
    }
}
pub fn parse(provider: &str, body: &[u8]) -> Result<String> {
    let value: Value = serde_json::from_slice(body).context("invalidResponse")?;
    let result = match provider {
        "google" => value[0]
            .as_array()
            .context("invalidResponse")?
            .iter()
            .map(|v| v[0].as_str().context("invalidResponse"))
            .collect::<Result<Vec<_>>>()?
            .concat(),
        "mymemory" => {
            if value["responseStatus"] != 200 && value["responseStatus"] != "200" {
                bail!("rateLimited");
            }
            decode_entities(
                value["responseData"]["translatedText"]
                    .as_str()
                    .context("invalidResponse")?,
            )
        }
        _ => value["translatedText"]
            .as_str()
            .context("invalidResponse")?
            .to_owned(),
    };
    if result.trim().is_empty() || result.len() > 100000 {
        bail!("invalidResponse");
    }
    Ok(result)
}
fn decode_entities(input: &str) -> String {
    let mut result = String::new();
    let mut rest = input;
    while let Some(index) = rest.find('&') {
        result.push_str(&rest[..index]);
        rest = &rest[index..];
        let Some(end) = rest.find(';').filter(|n| *n < 16) else {
            result.push('&');
            rest = &rest[1..];
            continue;
        };
        let entity = &rest[1..end];
        let character = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => entity
                .strip_prefix("#x")
                .and_then(|v| u32::from_str_radix(v, 16).ok())
                .or_else(|| entity.strip_prefix('#').and_then(|v| v.parse().ok()))
                .and_then(char::from_u32),
        };
        if let Some(c) = character {
            result.push(c);
            rest = &rest[end + 1..];
        } else {
            result.push('&');
            rest = &rest[1..];
        }
    }
    result.push_str(rest);
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_rejects_bad_payloads() {
        assert!(parse("google", b"<html>limit</html>").is_err());
        assert!(parse("google", b"[[]]").is_err());
        assert!(parse("libre", b"{}").is_err());
    }
    #[test]
    fn markup_remains_literal() {
        assert_eq!(parse("mymemory",br#"{"responseStatus":200,"responseData":{"translatedText":"&lt;img src=\"x\"&gt; &amp; &#1040;"}}"#).unwrap(),"<img src=\"x\"> & А");
    }
    #[test]
    fn google_concatenates_sentences() {
        assert_eq!(
            parse("google", br#"[[["Hello ","a"],["world","b"]],null,"en"]"#).unwrap(),
            "Hello world"
        );
    }
}
