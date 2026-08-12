//! Paths, names, and ports for the RLBot v5 installation.
//!
//! File structure of RLBot v5 on Linux:
//!
//! ```text
//! $XDG_DATA_HOME/RLBot5/
//!   bin/
//!     RLBotServer   # runs on the HOST via flatpak-spawn (must see /proc and spawn the game)
//!     rlbotgui      # runs inside the sandbox
//!   logs/
//!     rlbot.log
//!     rlbotserver.log
//!   botpacks/
//!   local/
//! ```
//!
//! Inside the flatpak, $XDG_DATA_HOME points at ~/.var/app/org.rlbot.gui/data,
//! which is a real host path, so the host-side server can execute from it too.

use anyhow::Context;
use directories::BaseDirs;
use std::path::PathBuf;

pub const RLBOT_BIN_DIR: &str = "RLBot5/bin";
pub const RLBOT_LOG_DIR: &str = "RLBot5/logs";
pub const RLBOT_GUI_BIN_NAME: &str = "rlbotgui";
pub const RLBOT_SERVER_BIN_NAME: &str = "RLBotServer";
pub const RLBOT_SERVER_PORT: u16 = 23234;

/// Name of this launcher's own process, used to track it from the host.
pub const RLBOT_LAUNCHER_NAME: &str = "rlbot-launcher";

/// Repo names under the `RLBot` GitHub org, used to build the release URLs.
pub const RLBOT_GUI_REPO_NAME: &str = "gui";
pub const RLBOT_SERVER_REPO_NAME: &str = "core";

/// Directory holding the downloaded server and GUI binaries.
pub fn bin_dir() -> anyhow::Result<PathBuf> {
    Ok(data_local_dir()?.join(RLBOT_BIN_DIR))
}

/// Directory holding the launcher and RLBotServer log files.
pub fn log_dir() -> anyhow::Result<PathBuf> {
    Ok(data_local_dir()?.join(RLBOT_LOG_DIR))
}

fn data_local_dir() -> anyhow::Result<PathBuf> {
    let base_dirs = BaseDirs::new().context("Could not get BaseDirs")?;
    Ok(base_dirs.data_local_dir().to_path_buf())
}
