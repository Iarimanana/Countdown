//! Makes sure only one copy of the app runs at a time, so two windows never
//! overwrite each other's device list.

use std::fs::{self, File, TryLockError};
use std::path::Path;

/// Held for as long as the app runs; the OS releases the lock when the
/// process ends, even after a crash.
pub struct InstanceLock {
    _file: File,
}

pub enum Acquire {
    Acquired(InstanceLock),
    /// Another copy of the app is already running.
    AlreadyRunning,
    /// Locking isn't possible here (unusual file system). The app still runs.
    Unavailable,
}

pub fn acquire(dir: &Path) -> Acquire {
    if fs::create_dir_all(dir).is_err() {
        return Acquire::Unavailable;
    }
    let Ok(file) = File::options().create(true).truncate(false).write(true).open(dir.join("countdown.lock")) else {
        return Acquire::Unavailable;
    };
    match file.try_lock() {
        Ok(()) => Acquire::Acquired(InstanceLock { _file: file }),
        Err(TryLockError::WouldBlock) => Acquire::AlreadyRunning,
        Err(TryLockError::Error(_)) => Acquire::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_lock_is_refused_until_first_is_released() {
        let dir = tempfile::tempdir().unwrap();
        let first = acquire(dir.path());
        assert!(matches!(first, Acquire::Acquired(_)));
        assert!(matches!(acquire(dir.path()), Acquire::AlreadyRunning));
        drop(first);
        assert!(matches!(acquire(dir.path()), Acquire::Acquired(_)));
    }

    #[test]
    fn unusable_folder_does_not_block_the_app() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("not-a-folder");
        fs::write(&file, "x").unwrap();
        assert!(matches!(acquire(&file), Acquire::Unavailable));
    }
}
