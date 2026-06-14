//! `stpl init` — create the config file.
//!
//! CONTRACT — implement `run`; do not change its signature.

use anyhow::Result;

use crate::{config, output};

/// Write the default config file and report the path. If config already
/// exists, surface `StplError::ConfigExists`. Reports what it did via
/// `output::success`.
pub fn run() -> Result<()> {
    let path = config::init()?;
    // The file was just written with defaults; report against those.
    let style = output::Style::from_config(&config::Config::default());
    output::success(&style, &format!("created config at {}", path.display()));
    Ok(())
}
