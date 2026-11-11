use anyhow::Result;
use std::{
    fs::{self, File, OpenOptions},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};
use tokio::net::{UnixListener, UnixStream};

pub type ServerStream = UnixStream;
pub type ClientStream = UnixStream;

pub struct Listener {
    inner: UnixListener,
    path: PathBuf,
}

impl Listener {
    /// Replaces a stale socket file and restricts the new one to the current user.
    pub fn bind(path: &Path) -> Result<Self> {
        let _ = fs::remove_file(path);
        let inner = UnixListener::bind(path)?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
        Ok(Self {
            inner,
            path: path.to_owned(),
        })
    }
    pub async fn accept(&mut self) -> Result<ServerStream> {
        Ok(self.inner.accept().await?.0)
    }
    /// Removes the socket file once the daemon stops listening.
    pub fn close(self) -> Result<()> {
        drop(self.inner);
        fs::remove_file(&self.path)?;
        Ok(())
    }
}

pub async fn connect(path: &Path) -> std::io::Result<ClientStream> {
    UnixStream::connect(path).await
}

pub fn create_private(path: &Path) -> Result<File> {
    Ok(OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .mode(0o600)
        .open(path)?)
}
