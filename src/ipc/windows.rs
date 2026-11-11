use anyhow::Result;
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    path::Path,
    time::Duration,
};
use tokio::net::windows::named_pipe::{
    ClientOptions, NamedPipeClient, NamedPipeServer, ServerOptions,
};

pub type ServerStream = NamedPipeServer;
pub type ClientStream = NamedPipeClient;

/// ERROR_PIPE_BUSY: every server instance is taken; the next one appears shortly.
const PIPE_BUSY: i32 = 231;

/// Pipe names share one machine-wide namespace. The socket path lives in the
/// user's own temp or runtime directory, so hashing it keeps names per user
/// and per display without putting the user name in the pipe name.
fn pipe_name(path: &Path) -> String {
    let digest = Sha256::digest(path.as_os_str().as_encoded_bytes());
    let hex: String = digest[..8].iter().map(|b| format!("{b:02x}")).collect();
    format!(r"\\.\pipe\omatranslate-{hex}")
}

/// Windows accepts on a pipe instance and creates the next instance before
/// handing the connected one out, so a client never finds the name missing.
pub struct Listener {
    name: String,
    next: NamedPipeServer,
}

impl Listener {
    pub fn bind(path: &Path) -> Result<Self> {
        let name = pipe_name(path);
        // first_pipe_instance fails if another process already owns the name,
        // so nobody can create the pipe first and intercept requests.
        let next = ServerOptions::new()
            .first_pipe_instance(true)
            .reject_remote_clients(true)
            .create(&name)?;
        Ok(Self { name, next })
    }
    pub async fn accept(&mut self) -> Result<ServerStream> {
        self.next.connect().await?;
        let next = ServerOptions::new()
            .reject_remote_clients(true)
            .create(&self.name)?;
        Ok(std::mem::replace(&mut self.next, next))
    }
    /// A pipe disappears with its last handle; there is no file to remove.
    pub fn close(self) -> Result<()> {
        Ok(())
    }
}

pub async fn connect(path: &Path) -> std::io::Result<ClientStream> {
    let name = pipe_name(path);
    for _ in 0..50 {
        match ClientOptions::new().open(&name) {
            Err(e) if e.raw_os_error() == Some(PIPE_BUSY) => {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            result => return result,
        }
    }
    ClientOptions::new().open(&name)
}

/// Files under the user's profile already inherit an owner-only ACL.
pub fn create_private(path: &Path) -> Result<File> {
    Ok(OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)?)
}
