//! Local IPC transport and the single-daemon lock.
//!
//! Unix uses a socket file with mode 0600 in the runtime directory. Windows uses a
//! named pipe derived from the same path, so `config::socket_path()` stays the one
//! place that decides where the daemon listens. Both sides speak the same framing.

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::{ClientStream, Listener, ServerStream, connect, create_private};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::{ClientStream, Listener, ServerStream, connect, create_private};

use anyhow::Result;
use std::{fs::File, path::Path};

/// Held for the daemon's lifetime; a second daemon fails to take it and exits.
pub struct Lock(#[allow(dead_code)] File);

/// Takes the exclusive, non-blocking lock next to the socket path.
/// Returns `None` when another daemon already holds it.
pub fn try_lock(path: &Path) -> Result<Option<Lock>> {
    let file = create_private(&path.with_extension("lock"))?;
    match file.try_lock() {
        Ok(()) => Ok(Some(Lock(file))),
        Err(std::fs::TryLockError::WouldBlock) => Ok(None),
        Err(std::fs::TryLockError::Error(e)) => Err(e.into()),
    }
}
