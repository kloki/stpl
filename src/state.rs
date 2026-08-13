//! Persistent "last memo used" pointer.
//!
//! CONTRACT — implement the bodies; do not change public signatures.
//!
//! Commands that target a single memo remember it here so the title can be
//! omitted next time (`stpl show`, `stpl edit`, …). Only the absolute path is
//! stored; the `Memo` is rehydrated with `Memo::from_path`, which makes a stale
//! pointer (memo deleted or renamed outside stpl) self-detecting — `load`
//! simply returns `None`.
//!
//! Every function here is best-effort: this is a convenience cache, never a
//! source of truth, so I/O errors are swallowed rather than failing a command.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::memo::Memo;

/// On-disk shape of the state file.
#[derive(Debug, Serialize, Deserialize)]
struct LastMemo {
    path: PathBuf,
}

/// Path to the state file: `~/.local/state/stpl/last.toml` (XDG state dir),
/// falling back to the local data dir on platforms without one. `None` when
/// neither can be determined.
pub fn path() -> Option<PathBuf> {
    let dir = dirs::state_dir().or_else(dirs::data_local_dir)?;
    Some(dir.join("stpl").join("last.toml"))
}

/// Remember `memo_path` as the last memo used. Best-effort: creates the state
/// directory lazily and silently does nothing if anything fails.
pub fn record(memo_path: &Path) {
    let Some(file) = path() else { return };
    record_at(&file, memo_path);
}

/// Load the remembered memo, or `None` when nothing is recorded, the state file
/// is unreadable/unparseable, or the memo no longer exists on disk.
pub fn load() -> Option<Memo> {
    load_at(&path()?)
}

/// Forget the remembered memo, but only if it currently points at `memo_path`.
/// Used by `del` so deleting a memo other than the remembered one leaves the
/// pointer intact.
pub fn clear_if(memo_path: &Path) {
    let Some(file) = path() else { return };
    if read_at(&file).as_deref() == Some(memo_path) {
        let _ = fs::remove_file(&file);
    }
}

/// `record` against an explicit state-file path (testable core).
fn record_at(file: &Path, memo_path: &Path) {
    if let Some(parent) = file.parent()
        && fs::create_dir_all(parent).is_err()
    {
        return;
    }
    let last = LastMemo {
        path: memo_path.to_path_buf(),
    };
    if let Ok(text) = toml::to_string_pretty(&last) {
        let _ = fs::write(file, text);
    }
}

/// `load` against an explicit state-file path (testable core).
///
/// `Memo::from_path` only parses the name, so existence is checked here — that
/// is what makes a pointer to a deleted/moved memo resolve to `None`.
fn load_at(file: &Path) -> Option<Memo> {
    let memo_path = read_at(file)?;
    if !memo_path.is_file() {
        return None;
    }
    Memo::from_path(&memo_path)
}

/// Read the recorded path out of `file`, without validating that it exists.
fn read_at(file: &Path) -> Option<PathBuf> {
    let text = fs::read_to_string(file).ok()?;
    let last: LastMemo = toml::from_str(&text).ok()?;
    Some(last.path)
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    /// A unique scratch directory per test, mirroring the store/config tests.
    fn temp_dir(name: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!("stpl-state-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn record_then_read_round_trips() {
        let dir = temp_dir("round-trip");
        let file = dir.join("nested").join("last.toml");
        let memo = dir.join("2026/33/2026-08-13-standup.md");

        record_at(&file, &memo);

        assert_eq!(read_at(&file), Some(memo));
    }

    #[test]
    fn read_missing_file_is_none() {
        let dir = temp_dir("missing");
        assert_eq!(read_at(&dir.join("last.toml")), None);
    }

    #[test]
    fn read_garbage_file_is_none() {
        let dir = temp_dir("garbage");
        let file = dir.join("last.toml");
        fs::write(&file, "not toml at all: [[[").unwrap();
        assert_eq!(read_at(&file), None);
    }

    #[test]
    fn load_of_stale_pointer_is_none() {
        let dir = temp_dir("stale");
        let file = dir.join("last.toml");
        // Recorded, but the memo was never created (or has since been deleted).
        record_at(&file, &dir.join("2026/33/2026-08-13-gone.md"));
        assert!(load_at(&file).is_none());
    }

    #[test]
    fn load_returns_the_recorded_memo() {
        let dir = temp_dir("load");
        let file = dir.join("last.toml");
        let week = dir.join("2026/33");
        fs::create_dir_all(&week).unwrap();
        let memo_path = week.join("2026-08-13-standup-notes.md");
        fs::write(&memo_path, "# Standup Notes\n").unwrap();

        record_at(&file, &memo_path);

        let memo = load_at(&file).expect("memo should load");
        assert_eq!(memo.slug, "standup-notes");
        assert_eq!(memo.path, memo_path);
    }
}
