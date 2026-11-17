//! Advisory file locks, so the command line and the app (or two runs) do not overwrite each other's
//! files. A lock is held until the returned file is dropped.

use std::fs::{File, OpenOptions, TryLockError};
use std::path::Path;

fn open(lock_path: &Path) -> Option<File> {
    if let Some(dir) = lock_path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    OpenOptions::new().create(true).truncate(false).write(true).open(lock_path).ok()
}

/// Wait for an exclusive lock on `lock_path`. `None` if it cannot be made or taken: callers carry on
/// without it, since a missing lock only risks the lost update it was there to prevent.
pub fn lock(lock_path: &Path) -> Option<File> {
    let file = open(lock_path)?;
    file.lock().ok()?;
    Some(file)
}

/// Take an exclusive lock on `lock_path` without waiting: `Ok(None)` if it cannot be made at all,
/// `Err(())` if somebody else holds it.
pub fn try_lock(lock_path: &Path) -> Result<Option<File>, ()> {
    let Some(file) = open(lock_path) else { return Ok(None) };
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(TryLockError::WouldBlock) => Err(()),
        Err(TryLockError::Error(_)) => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_held_lock_blocks_try_lock_until_dropped() {
        let dir = std::env::temp_dir().join(format!("pampas-lock-test-{}", std::process::id()));
        let path = dir.join("x.lock");
        let held = lock(&path).unwrap();
        assert!(try_lock(&path).is_err());
        drop(held);
        assert!(try_lock(&path).unwrap().is_some());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
