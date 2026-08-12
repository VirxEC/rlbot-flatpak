//! rlbot-launcher: keeps RLBotServer and rlbotgui in sync from their GitHub
//! releases, starts the server on the host, and runs the GUI.

mod config;
mod github;
mod server;
mod updates;

use crate::config::{RLBOT_GUI_BIN_NAME, RLBOT_SERVER_BIN_NAME, RLBOT_SERVER_PORT, bin_dir};
use anyhow::Context;
use clap::Parser;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, IsTerminal, Write};
use std::process::Command;
use std::sync::Mutex;
use tracing::{error, info, warn};

/// Launcher for RLBotGUI
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Force update the gui
    #[arg(short = 'g', long)]
    force_update_gui: bool,

    /// Force update the server
    #[arg(short = 's', long)]
    force_update_server: bool,

    /// Run as if offline
    #[arg(short, long, conflicts_with = "online")]
    offline: bool,

    /// Skip the internet connection check and pretend to be online
    #[arg(long, conflicts_with = "offline")]
    online: bool,
}

fn run() -> anyhow::Result<()> {
    let args = Args::parse();

    info!("Checking for internet connection...");
    let is_online = if args.offline {
        info!("Offline mode requested: skipping the connection check");
        false
    } else if args.online {
        info!("Online mode requested: assuming the connection is up");
        true
    } else {
        github::is_online()
    };
    info!("Is online: {is_online}");

    let bin_dir = bin_dir()?;
    fs::create_dir_all(&bin_dir)?;

    let gui_exists = bin_dir.join(RLBOT_GUI_BIN_NAME).exists();
    let server_exists = bin_dir.join(RLBOT_SERVER_BIN_NAME).exists();
    if is_online || !gui_exists || !server_exists {
        updates::check_for_updates(&bin_dir, args.force_update_gui, args.force_update_server);
    }

    let server_was_running = server::is_running();
    let server_path = bin_dir.join(RLBOT_SERVER_BIN_NAME);
    if server_was_running {
        info!("RLBotServer is already listening on 127.0.0.1:{RLBOT_SERVER_PORT}");
    } else if server_path.exists() {
        let server_log = config::log_dir()?.join("rlbotserver.log");
        match server::spawn_on_host(&server_path, &server_log) {
            Ok(()) => info!("Started RLBotServer on the host"),
            Err(e) => error!("Failed to start RLBotServer: {e:#}"),
        }
    } else {
        warn!("RLBotServer not found and could not be downloaded");
    }

    let gui_path = bin_dir.join(RLBOT_GUI_BIN_NAME);
    let status = Command::new(&gui_path)
        .current_dir(env::temp_dir())
        .status()
        .with_context(|| format!("Could not run rlbotgui at {gui_path:?}"))?;
    if !status.success() {
        warn!("rlbotgui exited with status {status}");
    }

    if !server_was_running {
        match server::kill_on_host() {
            Ok(()) => info!("Stopped RLBotServer"),
            Err(e) => warn!("Failed to stop RLBotServer: {e:#}"),
        }
    }

    Ok(())
}

/// The flatpak entry point has no console, so failures are also echoed to
/// stderr; the non-zero exit lets a session launched from a terminal notice.
fn main() -> std::process::ExitCode {
    init_logging();

    // Desktop launches have no terminal attached; open one on the host tailing
    // the logs so the updater and server output stay visible.
    if !io::stderr().is_terminal()
        && let Err(e) = server::open_log_terminal()
    {
        warn!("Could not open log terminal: {e:#}");
    }

    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            error!("{e:#}");
            eprintln!("rlbot-launcher: {e:#}");
            std::process::ExitCode::FAILURE
        }
    }
}

/// Set up tracing, writing the launcher and server logs fresh for this run
/// (mirroring the Windows launcher's per-run console). Falls back to stderr
/// only if the log files can't be opened.
fn init_logging() {
    let echo = io::stderr().is_terminal();
    match open_launcher_log() {
        Ok(file) => {
            tracing_subscriber::fmt()
                .with_ansi(true)
                .with_writer(Mutex::new(TeeWriter { file, echo }))
                .init();
        }
        Err(e) => {
            eprintln!("rlbot-launcher: could not set up logging: {e:#}");
            tracing_subscriber::fmt().init();
        }
    }
}

fn open_launcher_log() -> anyhow::Result<fs::File> {
    let dir = config::log_dir()?;
    fs::create_dir_all(&dir)?;
    let _ = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(dir.join("rlbotserver.log"));
    Ok(OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(dir.join("rlbot.log"))?)
}

/// Write tracing events to the log file, echoing them to stderr when the
/// launcher was started from a terminal.
struct TeeWriter {
    file: fs::File,
    echo: bool,
}

impl Write for TeeWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.file.write(buf)?;
        if self.echo {
            let _ = io::stderr().write(&buf[..n]);
        }
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}
