//! Minimal GitHub Releases client.
//!
//! Kept close to the Windows launcher (github.com/RLBot/launcher) so the
//! update logic behaves identically on both platforms.

use anyhow::Context;
use serde::Deserialize;
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

pub const USER_AGENT: &str = "rlbot-launcher";

#[derive(Debug, Deserialize)]
pub struct Asset {
    pub name: String,
    /// GitHub asset digest, e.g. `sha256:<hex>`.
    pub digest: String,
    pub browser_download_url: String,
}

#[derive(Debug, Deserialize)]
pub struct Release {
    pub name: String,
    pub assets: Vec<Asset>,
}

/// Fetch the latest release of `RLBot/{repo}`.
///
/// The `/releases/latest` endpoint excludes drafts and prereleases.
pub fn latest_release(repo: &str) -> anyhow::Result<Release> {
    let url = format!("https://api.github.com/repos/RLBot/{repo}/releases/latest");
    let response = ureq::get(&url)
        .header("User-Agent", USER_AGENT)
        .call()
        .with_context(|| format!("Could not get latest release of RLBot/{repo}"))?;

    let text = response.into_body().read_to_string()?;
    serde_json::from_str(&text)
        .with_context(|| format!("Could not parse latest release of RLBot/{repo}"))
}

/// Best-effort internet check: a fast raw TCP probe first, then an HTTPS probe
/// of the API endpoint we actually depend on. The TCP probe is fragile (IPv6-
/// only networks, proxy-only egress, blocked 1.1.1.1), so it's bound with a
/// timeout and the HTTPS probe makes filtered networks fail fast too.
pub fn is_online() -> bool {
    if let Ok(addr) = "1.1.1.1:80".parse::<SocketAddr>()
        && TcpStream::connect_timeout(&addr, Duration::from_secs(3)).is_ok()
    {
        return true;
    }

    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(5)))
        .build()
        .new_agent();
    agent
        .get("https://api.github.com")
        .header("User-Agent", USER_AGENT)
        .call()
        .is_ok()
}
