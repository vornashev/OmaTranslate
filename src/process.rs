use anyhow::{Context, Result, bail};
use std::process::Stdio;
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
};
use tokio_util::sync::CancellationToken;

pub async fn run(
    program: &str,
    args: &[&str],
    input: &[u8],
    seconds: u64,
    cancel: &CancellationToken,
) -> Result<Vec<u8>> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .context("missingTool")?;
    let mut stdin = child.stdin.take().unwrap();
    let bytes = input.to_vec();
    let writer = tokio::spawn(async move {
        if !bytes.is_empty() {
            let _ = stdin.write_all(&bytes).await;
        }
    });
    let mut stdout = child.stdout.take().unwrap().take(64 * 1024 * 1024 + 1);
    let work = async {
        let mut result = Vec::new();
        stdout.read_to_end(&mut result).await?;
        if result.len() > 64 * 1024 * 1024 {
            bail!("inputTooLarge");
        }
        let status = child.wait().await?;
        if !status.success() {
            bail!("toolFailed");
        }
        Ok(result)
    };
    let result = tokio::select! {
        _=cancel.cancelled()=>Err(anyhow::anyhow!("cancelled")),
        r=tokio::time::timeout(Duration::from_secs(seconds),work)=>r.unwrap_or_else(|_|Err(anyhow::anyhow!("timeout"))),
    };
    writer.abort();
    result
}
