mod config;
mod protect;
mod images;
mod languages;
mod models;
mod portal;
mod process;
mod providers;
mod server;
use anyhow::{Context, Result, bail};
use serde_json::json;
use std::{process::Stdio, time::Duration};
use tokio::{
    io::{AsyncWriteExt, BufReader},
    net::UnixStream,
    process::Command,
};

#[tokio::main]
async fn main() {
    if let Err(e) = cli().await {
        eprintln!("{}", server::error_code(&e));
        std::process::exit(1);
    }
}
async fn cli() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("open");
    if ["--help", "help", "-h"].contains(&command) {
        println!(
            "OmaTranslate\n\n  open | settings | selection | clipboard | screenshot\n  image <path>               Open and translate an image\n  translate <text>           Translate and print JSON\n  status | doctor            Report state or dependencies as JSON\n  serve                      Run the background service\n  request '<json>'           Send a versioned IPC request\n\nConfiguration: $XDG_CONFIG_HOME/omatranslate/settings.json\nModels: $XDG_DATA_HOME/omatranslate/models"
        );
        return Ok(());
    }
    if command == "serve" {
        return if args.iter().any(|a| a == "--stdio") {
            server::serve_stdio().await
        } else {
            server::serve().await
        };
    }
    if command == "doctor" {
        let mut checks = serde_json::Map::new();
        for tool in [
            "wl-paste",
            "wl-copy",
            "grim",
            "slurp",
            "tesseract",
            "omarchy-shell",
        ] {
            let available = std::env::var_os("PATH")
                .map(|p| std::env::split_paths(&p).any(|d| d.join(tool).is_file()))
                .unwrap_or(false);
            checks.insert(tool.into(), json!(available));
        }
        let langs = process::run(
            "tesseract",
            &["--list-langs"],
            &[],
            5,
            &tokio_util::sync::CancellationToken::new(),
        )
        .await
        .unwrap_or_default();
        println!(
            "{}",
            json!({"version":env!("CARGO_PKG_VERSION"),"tools":checks,"ocrLanguages":String::from_utf8_lossy(&langs).lines().skip(1).collect::<Vec<_>>(),"offlineEngine":models::engine_available().await,"models":models::status(),"socket":config::socket_path()})
        );
        return Ok(());
    }
    let mut socket = connect().await?;
    let request = match command {
        "request" => serde_json::from_str(args.get(1).context("invalidRequest")?)?,
        "translate" => {
            json!({"v":1,"op":"translate","text":args.get(1).context("invalidRequest")?})
        }
        "image" => {
            json!({"v":1,"op":"image","path":std::fs::canonicalize(args.get(1).context("invalidImage")?).context("invalidImage")?})
        }
        "open" | "settings" | "selection" | "clipboard" | "screenshot" | "replace" | "status" => {
            json!({"v":1,"op":command})
        }
        _ => bail!("unknownOperation"),
    };
    let waiting = command == "translate";
    if waiting {
        socket
            .write_all(b"{\"v\":1,\"op\":\"subscribe\"}\n")
            .await?;
    }
    socket.write_all(format!("{request}\n").as_bytes()).await?;
    let mut reader = BufReader::new(socket);
    let mut job = None;
    loop {
        let value = tokio::time::timeout(Duration::from_secs(120), server::frame(&mut reader))
            .await
            .context("timeout")??
            .context("connectionClosed")?;
        if !waiting {
            println!("{value}");
            if value["event"] == "error" {
                bail!("requestFailed");
            }
            return Ok(());
        }
        if value["event"] == "ack" {
            job = value["job"].as_u64();
        }
        if value["event"] == "error" {
            println!("{value}");
            bail!("requestFailed");
        }
        if job.is_some()
            && value["job"].as_u64() == job
            && ["ready", "error"].contains(&value["status"].as_str().unwrap_or(""))
        {
            println!("{value}");
            if value["status"] == "error" {
                bail!("translationFailed");
            }
            return Ok(());
        }
    }
}
async fn connect() -> Result<UnixStream> {
    let path = config::socket_path();
    if let Ok(socket) = UnixStream::connect(&path).await {
        return Ok(socket);
    }
    Command::new(std::env::current_exe()?)
        .arg("serve")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("serviceUnavailable")?;
    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(20)).await;
        if let Ok(socket) = UnixStream::connect(&path).await {
            return Ok(socket);
        }
    }
    bail!("serviceUnavailable")
}
