//! Keep the RLBotServer/rlbotgui pair in sync by downloading them from their
//! GitHub releases.

use crate::config::{
    RLBOT_GUI_BIN_NAME, RLBOT_GUI_REPO_NAME, RLBOT_SERVER_BIN_NAME, RLBOT_SERVER_REPO_NAME,
};
use crate::github;
use anyhow::{Context, anyhow};
use sha256::digest;
use std::fs;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use tracing::{error, info};

/// Check both repos in the same pass so the pair stays in sync: an update to
/// either one triggers a check of both. The two fetches run in parallel and
/// failures are logged but never abort the other fetch or the launch.
///
/// The caller runs this whenever online, or whenever either binary is missing,
/// so a fresh install still gets both binaries even without a connection.
pub fn check_for_updates(bin_dir: &Path, force_gui: bool, force_server: bool) {
    let (gui, server) = std::thread::scope(|scope| {
        let gui = scope
            .spawn(|| update_binary(bin_dir, RLBOT_GUI_BIN_NAME, RLBOT_GUI_REPO_NAME, force_gui));
        let server = scope.spawn(|| {
            update_binary(
                bin_dir,
                RLBOT_SERVER_BIN_NAME,
                RLBOT_SERVER_REPO_NAME,
                force_server,
            )
        });
        (gui.join(), server.join())
    });

    report("gui", gui);
    report("server", server);
}

/// Download `bin_name` from the latest release of `RLBot/{repo}` into
/// `bin_dir` when it differs from the cached copy (or is missing), verified
/// against the release asset's sha256 digest.
fn update_binary(bin_dir: &Path, bin_name: &str, repo: &str, force: bool) -> anyhow::Result<()> {
    let bin_path = bin_dir.join(bin_name);
    let local_sha = fs::read(&bin_path).ok().map(digest);

    let release = github::latest_release(repo)?;
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == bin_name)
        .ok_or_else(|| anyhow!("Could not find {bin_name} asset in latest release"))?;
    let asset_sha = parse_digest(&asset.digest)?;

    if !force && local_sha.as_deref() == Some(asset_sha) {
        info!("{bin_name} is up to date");
        return Ok(());
    }
    if force {
        info!("Forcing update of {bin_name} ...");
    }

    info!("Downloading latest {bin_name} ({})...", release.name);
    let response = ureq::get(&asset.browser_download_url)
        .header("User-Agent", github::USER_AGENT)
        .call()
        .with_context(|| format!("Could not download {bin_name}"))?;

    info!("Applying update to {bin_name} ...");
    let mut bytes = Vec::new();
    response.into_body().into_reader().read_to_end(&mut bytes)?;

    fs::write(&bin_path, bytes)?;
    make_executable(&bin_path)?;

    Ok(())
}

/// Linux needs the executable bit on downloaded binaries.
fn make_executable(path: &Path) -> anyhow::Result<()> {
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms)?;
    Ok(())
}

/// Parse a GitHub asset digest like `sha256:<hex>`.
fn parse_digest(digest: &str) -> anyhow::Result<&str> {
    digest
        .strip_prefix("sha256:")
        .ok_or_else(|| anyhow!("GitHub digest does not start with 'sha256:': {digest:?}"))
}

/// Log the outcome of one update thread.
fn report(what: &str, result: std::thread::Result<anyhow::Result<()>>) {
    match result {
        Ok(Ok(())) => {}
        Ok(Err(e)) => error!("Failed to update {what}: {e:#}"),
        Err(_) => error!("{what} update thread panicked"),
    }
}
