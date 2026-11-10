//! Exercises the real daemon over IPC, including actual Tesseract and PNG rendering.
//!
//! Only translation inference is replaced by a deterministic worker fixture, so this
//! says nothing about translation quality or model speed. Each test starts its own
//! daemon in a private temporary home.

use serde_json::{json, Value};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

const WORKER: &str = r#"import json,sys,time
print(json.dumps({"event":"ready"}),flush=True)
for line in sys.stdin:
 r=json.loads(line)
 if any("slow" in x for x in r["segments"]): time.sleep(0.7)
 if any("fail" in x for x in r["segments"]): answer={"error":"offlineFailed"}
 else: answer={"translations":["Перевод: "+x for x in r["segments"]]}
 print(json.dumps(answer,ensure_ascii=False),flush=True)
"#;

struct Daemon {
    root: tempfile::TempDir,
    socket: Option<PathBuf>,
    child: Child,
    reader: Box<dyn BufRead>,
    writer: Box<dyn Write>,
}

impl Daemon {
    fn start() -> Self {
        let root = tempfile::Builder::new().prefix("omatranslate-test-").tempdir().unwrap();
        let socket = root.path().join("service.sock");
        // Some sandboxes forbid Unix sockets; the stdio transport shares the dispatcher.
        let stdio = UnixListener::bind(root.path().join("probe.sock")).is_err();
        if stdio {
            eprintln!("Socket creation denied: using the stdio transport; the Unix transport is not exercised.");
        }
        let worker = root.path().join("worker.py");
        fs::write(&worker, WORKER).unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_omatranslate"));
        command
            .arg("serve")
            .env("XDG_CONFIG_HOME", root.path().join("config"))
            .env("XDG_DATA_HOME", root.path().join("data"))
            .env("OMATRANSLATE_SOCKET", &socket)
            .env("OMATRANSLATE_WORKER", &worker)
            .env("OMATRANSLATE_NO_SHELL", "1")
            .stderr(fs::File::create(root.path().join("stderr")).unwrap());
        if stdio {
            command.arg("--stdio").stdin(Stdio::piped()).stdout(Stdio::piped());
        } else {
            command.stdin(Stdio::null()).stdout(Stdio::null());
        }
        let mut child = command.spawn().expect("build the omatranslate binary");
        let (reader, writer): (Box<dyn BufRead>, Box<dyn Write>) = if stdio {
            (Box::new(BufReader::new(child.stdout.take().unwrap())), Box::new(child.stdin.take().unwrap()))
        } else {
            let deadline = Instant::now() + Duration::from_secs(5);
            let stream = loop {
                if let Ok(stream) = UnixStream::connect(&socket) {
                    break stream;
                }
                if let Some(status) = child.try_wait().unwrap() {
                    panic!("daemon exited with {status}: {}", fs::read_to_string(root.path().join("stderr")).unwrap());
                }
                assert!(Instant::now() < deadline, "daemon socket never appeared");
                sleep(Duration::from_millis(20));
            };
            stream.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
            (Box::new(BufReader::new(stream.try_clone().unwrap())), Box::new(stream))
        };
        let mut daemon = Daemon { root, socket: (!stdio).then_some(socket), child, reader, writer };
        daemon.request(json!({"op": "setSettings", "settings": {
            "provider": "local", "source": "en", "target": "ru", "ocrLanguages": "eng", "history": false}}));
        daemon.request(json!({"op": "dismiss"}));
        daemon
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.path().join(relative)
    }

    fn request(&mut self, mut body: Value) -> Value {
        body["v"] = json!(1);
        self.writer.write_all(format!("{body}\n").as_bytes()).unwrap();
        self.writer.flush().unwrap();
        let mut line = String::new();
        self.reader.read_line(&mut line).expect("daemon did not respond");
        assert!(!line.is_empty(), "daemon closed the connection");
        serde_json::from_str(&line).unwrap()
    }

    fn wait_state(&mut self, predicate: impl Fn(&Value) -> bool) -> Value {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            let state = self.request(json!({"op": "status"}));
            if predicate(&state) {
                return state;
            }
            assert!(Instant::now() < deadline, "state never reached: {state}");
            sleep(Duration::from_millis(30));
        }
    }

    fn translate(&mut self, text: &str) -> Value {
        self.request(json!({"op": "translate", "text": text}))
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn mode(path: &Path) -> u32 {
    fs::metadata(path).unwrap().permissions().mode() & 0o777
}

#[test]
fn translation_and_no_disk_history() {
    let mut d = Daemon::start();
    let job = d.translate("Hello world")["job"].clone();
    let state = d.wait_state(|s| s["job"] == job && s["status"] == "ready");
    assert_eq!(state["translated"], "Перевод: Hello world");
    assert!(!d.path("data/omatranslate/history.json").exists());
}

#[test]
fn newest_request_wins() {
    let mut d = Daemon::start();
    d.translate("slow old request");
    sleep(Duration::from_millis(50));
    let job = d.translate("new request")["job"].clone();
    let state = d.wait_state(|s| s["job"] == job && s["status"] == "ready");
    assert_eq!(state["translated"], "Перевод: new request");
    sleep(Duration::from_millis(800));
    assert_eq!(d.request(json!({"op": "status"}))["translated"], state["translated"]);
}

#[test]
fn image_ocr_render_export_and_cleanup() {
    let mut d = Daemon::start();
    // Copy the fixture so the "original is protected" check never touches the repository file.
    let fixture = d.path("ocr-input.png");
    fs::copy(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/ocr-input.png"), &fixture).unwrap();
    let job = d.request(json!({"op": "image", "path": fixture}))["job"].clone();
    let state = d.wait_state(|s| s["job"] == job && (s["status"] == "ready" || s["status"] == "error"));
    assert_eq!(state["status"], "ready", "{state}");
    assert!(state["sourceText"].as_str().unwrap().contains("Welcome"), "{state}");
    assert!(!state["blocks"].as_array().unwrap().is_empty());
    let rendered = PathBuf::from(state["translatedImage"].as_str().unwrap());
    let mut magic = [0u8; 8];
    fs::File::open(&rendered).unwrap().read_exact(&mut magic).unwrap();
    assert_eq!(&magic, b"\x89PNG\r\n\x1a\n");

    let output = d.path("export.png");
    assert_eq!(d.request(json!({"op": "save", "path": output}))["event"], "ack");
    assert_eq!(fs::read(&output).unwrap(), fs::read(&rendered).unwrap());
    assert_eq!(d.request(json!({"op": "save", "path": output}))["error"], "fileExists");
    let protected = d.request(json!({"op": "save", "path": fixture, "overwrite": true}));
    assert_eq!(protected["error"], "originalProtected");
    let too_large = d.request(json!({"op": "editBlock", "index": 0, "text": "x".repeat(10_001)}));
    assert_eq!(too_large["error"], "inputTooLarge");
    assert_eq!(d.request(json!({"op": "status"}))["status"], "ready");

    d.request(json!({"op": "editBlock", "index": 0, "text": "Corrected recognition"}));
    let state = d.wait_state(|s| s["status"] == "ready" && s["blocks"][0]["source"] == "Corrected recognition");
    assert!(state["blocks"][0]["translated"].as_str().unwrap().contains("Corrected recognition"));
    d.request(json!({"op": "dismiss"}));
    assert!(!rendered.exists(), "temporary render was not removed");
    assert!(output.exists(), "user export was removed");
}

#[test]
fn errors_preserve_source() {
    let mut d = Daemon::start();
    d.translate("fail this request");
    let state = d.wait_state(|s| s["status"] == "error");
    assert_eq!(state["sourceText"], "fail this request");
    assert_eq!(state["error"], "offlineFailed");
}

#[test]
fn history_opt_in_and_clear() {
    let mut d = Daemon::start();
    d.request(json!({"op": "setSettings", "settings": {"history": true}}));
    d.translate("Saved phrase");
    let state = d.wait_state(|s| s["status"] == "ready");
    assert_eq!(state["history"][0]["source"], "Saved phrase");
    d.request(json!({"op": "clearHistory"}));
    assert_eq!(d.request(json!({"op": "status"}))["history"], json!([]));
}

#[test]
fn validation_and_private_settings() {
    let mut d = Daemon::start();
    let reply = d.request(json!({"op": "setSettings", "settings": {"provider": "unexpected"}}));
    assert_eq!(reply["error"], "invalidProvider");
    assert_eq!(mode(&d.path("config/omatranslate/settings.json")), 0o600);
    if let Some(socket) = d.socket.clone() {
        assert_eq!(mode(&socket), 0o600);
    }
}
