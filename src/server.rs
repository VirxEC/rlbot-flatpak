//! RLBotServer process management.
//!
//! The server must run on the host so it can see the game in `/proc` and
//! spawn Steam/Proton and bots. Inside the flatpak we cross the sandbox
//! boundary with `flatpak-spawn --host`; outside a flatpak (dev/testing) we
//! spawn it directly.

use crate::config::{RLBOT_SERVER_BIN_NAME, RLBOT_SERVER_PORT};
use anyhow::{Context, ensure};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, IsTerminal, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

const FLATPAK_SPAWN: &str = "/usr/bin/flatpak-spawn";

/// True if something is already listening on the RLBot sockets port.
pub fn is_running() -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], RLBOT_SERVER_PORT));
    TcpStream::connect_timeout(&addr, Duration::from_secs(1)).is_ok()
}

/// Start RLBotServer as a background process, streaming its output to the
/// launcher's stderr (when a terminal is attached) and to `log_path`.
///
/// Inside a flatpak, `flatpak-spawn --host` runs the server on the host; the
/// process is spawned rather than waited on, so the launcher can keep running
/// the GUI while a background thread drains the server's output.
pub fn spawn_on_host(server_path: &Path, log_path: &Path) -> anyhow::Result<()> {
    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .with_context(|| format!("Could not open {} for writing", log_path.display()))?;
    let echo = io::stderr().is_terminal();

    let mut child = if Path::new(FLATPAK_SPAWN).exists() {
        Command::new(FLATPAK_SPAWN)
            .current_dir(env::temp_dir())
            .args(["--host", "sh", "-c", "exec \"$1\" \"$2\" 2>&1", "sh"])
            .arg(server_path)
            .arg(RLBOT_SERVER_PORT.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .with_context(|| format!("Could not run {FLATPAK_SPAWN}"))?
    } else {
        Command::new("sh")
            .args(["-c", "exec \"$1\" \"$2\" 2>&1", "sh"])
            .arg(server_path)
            .arg(RLBOT_SERVER_PORT.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .with_context(|| format!("Failed to spawn {server_path:?}"))?
    };

    if let Some(stdout) = child.stdout.take() {
        tee_output(stdout, log, echo);
    }
    std::thread::spawn(move || {
        let _ = child.wait();
    });

    Ok(())
}

/// Stop RLBotServer. `pkill` exits 1 when no process matched, which we treat
/// as success.
pub fn kill_on_host() -> anyhow::Result<()> {
    let status = if Path::new(FLATPAK_SPAWN).exists() {
        Command::new(FLATPAK_SPAWN)
            .args(["--host", "pkill", "-x", RLBOT_SERVER_BIN_NAME])
            .status()?
    } else {
        Command::new("pkill")
            .args(["-x", RLBOT_SERVER_BIN_NAME])
            .status()?
    };

    ensure!(
        status.success() || status.code() == Some(1),
        "pkill failed with status {status}"
    );
    Ok(())
}

/// Open a terminal on the host showing the current run's logs; it closes when
/// the launcher exits, like the Windows launcher's console. Best-effort: needs
/// `x-terminal-emulator` on the host.
pub fn open_log_terminal() -> anyhow::Result<()> {
    let log_dir = crate::config::log_dir()?;
    let launcher_log = log_dir.join("rlbot.log").to_string_lossy().into_owned();
    let server_log = log_dir
        .join("rlbotserver.log")
        .to_string_lossy()
        .into_owned();
    let script = format!(
        "pid=$(pgrep -x {name} | head -1); [ -n \"$pid\" ] && exec tail -n 100 -q -f --pid=\"$pid\" \"{launcher}\" \"{server}\"",
        name = crate::config::RLBOT_LAUNCHER_NAME,
        launcher = launcher_log,
        server = server_log,
    );
    let arg = format!("sh -c {}", shell_quote(&script));

    if Path::new(FLATPAK_SPAWN).exists() {
        Command::new(FLATPAK_SPAWN)
            .args(["--host", "x-terminal-emulator", "-e"])
            .arg(&arg)
            .spawn()
            .map(|_| ())
            .with_context(|| format!("Could not run {FLATPAK_SPAWN}"))?;
    } else {
        Command::new("x-terminal-emulator")
            .args(["-e"])
            .arg(&arg)
            .spawn()
            .map(|_| ())
            .with_context(|| "Could not run x-terminal-emulator")?;
    }
    Ok(())
}

/// Append `reader` line by line to `log`, echoing each line to stderr when a
/// terminal is attached.
fn tee_output(reader: impl Read + Send + 'static, mut log: fs::File, echo: bool) {
    std::thread::spawn(move || {
        for line in BufReader::new(reader).lines().map_while(Result::ok) {
            if writeln!(log, "{line}").is_err() {
                break;
            }
            if echo {
                let _ = writeln!(io::stderr(), "{line}");
            }
        }
    });
}

/// Single-quote `s` for safe use in a shell command line.
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
