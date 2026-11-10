use anyhow::{Context, Result, bail};
use futures_util::StreamExt;
use std::{collections::HashMap, path::PathBuf};
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

pub async fn save_path() -> Result<Option<PathBuf>> {
    let connection = zbus::Connection::session()
        .await
        .context("portalUnavailable")?;
    let sender = connection
        .unique_name()
        .context("portalUnavailable")?
        .as_str()
        .trim_start_matches(':')
        .replace('.', "_");
    let token = format!("omatranslate{}", std::process::id());
    let path = format!("/org/freedesktop/portal/desktop/request/{sender}/{token}");
    let request = zbus::Proxy::new(
        &connection,
        "org.freedesktop.portal.Desktop",
        path,
        "org.freedesktop.portal.Request",
    )
    .await?;
    let mut responses = request.receive_signal("Response").await?;
    let chooser = zbus::Proxy::new(
        &connection,
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.FileChooser",
    )
    .await
    .context("portalUnavailable")?;
    let mut options: HashMap<&str, Value<'_>> = HashMap::new();
    options.insert("handle_token", Value::from(token.as_str()));
    options.insert("current_name", Value::from("translation.png"));
    let _: OwnedObjectPath = chooser
        .call("SaveFile", &("", "OmaTranslate", options))
        .await
        .context("portalUnavailable")?;
    let message = responses.next().await.context("portalUnavailable")?;
    let (code, result): (u32, HashMap<String, OwnedValue>) = message.body().deserialize()?;
    if code != 0 {
        return Ok(None);
    }
    let uris = Vec::<String>::try_from(
        result
            .get("uris")
            .context("portalUnavailable")?
            .try_clone()?,
    )?;
    let uri = reqwest::Url::parse(uris.first().context("portalUnavailable")?)?;
    if uri.scheme() != "file" {
        bail!("unsupportedDestination");
    }
    Ok(Some(
        uri.to_file_path()
            .map_err(|_| anyhow::anyhow!("unsupportedDestination"))?,
    ))
}
