//! Configuration: `~/.config/stpl.toml` loading and `stpl init`.
//!
//! CONTRACT — implement the bodies; do not change public signatures.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::error::StplError;

/// Runtime configuration. `memo_directory` is always an absolute path (with
/// `~` expanded) by the time a `Config` exists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Root directory holding all memos. Default: `~/stpls`.
    #[serde(default = "default_memo_directory")]
    pub memo_directory: PathBuf,
    /// Disable ANSI color in output.
    #[serde(default)]
    pub disable_color: bool,
}

/// The default memo directory (`~/stpls`), absolute.
pub fn default_memo_directory() -> PathBuf {
    match dirs::home_dir() {
        Some(home) => home.join("stpls"),
        // Extremely unlikely; fall back to a relative path rather than panic.
        None => PathBuf::from("stpls"),
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            memo_directory: default_memo_directory(),
            disable_color: false,
        }
    }
}

impl Config {
    /// Path to the config file: `~/.config/stpl.toml`.
    pub fn path() -> Result<PathBuf> {
        let dir = dirs::config_dir().context("could not determine config directory (~/.config)")?;
        Ok(dir.join("stpl.toml"))
    }

    /// Load config from disk, falling back to defaults if the file is absent.
    /// Expands `~` in `memo_directory` to an absolute path. Does NOT create
    /// the memo directory.
    pub fn load() -> Result<Config> {
        let path = Self::path()?;
        let mut config = if path.exists() {
            let text = fs::read_to_string(&path)
                .with_context(|| format!("reading config '{}'", path.display()))?;
            toml::from_str(&text).with_context(|| format!("parsing config '{}'", path.display()))?
        } else {
            Config::default()
        };
        config.memo_directory = expand_tilde(&config.memo_directory);
        Ok(config)
    }
}

/// Expand a leading `~` in a path to the user's home directory. Paths without
/// a leading `~` are returned unchanged. Does NOT canonicalize (the directory
/// may not exist yet).
fn expand_tilde(path: &Path) -> PathBuf {
    let Ok(stripped) = path.strip_prefix("~") else {
        return path.to_path_buf();
    };
    match dirs::home_dir() {
        Some(home) => home.join(stripped),
        None => path.to_path_buf(),
    }
}

/// Implementation of `stpl init`: write a default config file. Errors with
/// `StplError::ConfigExists` if one is already present. Returns the path
/// written. Does NOT create the memo directory (lazy).
pub fn init() -> Result<PathBuf> {
    let path = Config::path()?;
    if path.exists() {
        return Err(StplError::ConfigExists(path).into());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating config directory '{}'", parent.display()))?;
    }
    let text = toml::to_string_pretty(&Config::default()).context("serializing default config")?;
    fs::write(&path, text).with_context(|| format!("writing config '{}'", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_memo_directory_is_absolute() {
        // When a home dir is available it must be absolute and end in `stpls`.
        if dirs::home_dir().is_some() {
            let dir = default_memo_directory();
            assert!(dir.is_absolute());
            assert!(dir.ends_with("stpls"));
        }
    }

    #[test]
    fn expand_tilde_expands_leading_tilde() {
        if let Some(home) = dirs::home_dir() {
            assert_eq!(expand_tilde(Path::new("~/stpls")), home.join("stpls"));
            assert_eq!(expand_tilde(Path::new("~")), home.clone());
        }
        // Absolute paths pass through unchanged.
        assert_eq!(
            expand_tilde(Path::new("/var/notes")),
            PathBuf::from("/var/notes")
        );
    }
}
