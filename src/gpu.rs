//! GPU workarounds the GUI needs before WebKitGTK starts.
//!
//! WebKitGTK renders through the DMA-BUF renderer on Linux, which maps
//! dma-bufs it imports from the GPU. The proprietary NVIDIA driver fails that
//! map silently, so the GUI comes up blank (webkit bug 262607 / 180739), and
//! `WEBKIT_DISABLE_DMABUF_RENDERER=1` makes it fall back to a path that works.
//! The runtime's WebKit only does that itself on some setups, so the launcher
//! sets the variable for the GUI when it detects the driver.

use crate::config::FLATPAK_SPAWN;
use std::env;
use std::path::Path;
use std::process::Command;

/// The WebKitGTK switch that turns off the DMA-BUF renderer.
pub const DMABUF_RENDERER_ENV: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";

/// Kernel module directory of the proprietary NVIDIA driver. Nouveau loads as
/// `nouveau`, so this only matches the driver that misbehaves.
const NVIDIA_MODULE_PATH: &str = "/sys/module/nvidia";

/// True when the GUI needs `WEBKIT_DISABLE_DMABUF_RENDERER=1` set on its
/// environment: the proprietary NVIDIA driver is loaded and the user has not
/// set the variable themselves.
///
/// Any value counts as the user having spoken, since WebKit treats a set
/// variable as "disabled" regardless of its contents. This is what makes
/// `flatpak override --user --env=WEBKIT_DISABLE_DMABUF_RENDERER=1` still work
/// on machines the check gets wrong.
pub fn needs_dmabuf_workaround() -> bool {
    if env::var_os(DMABUF_RENDERER_ENV).is_some() {
        return false;
    }
    nvidia_driver_loaded()
}

/// Whether the proprietary NVIDIA driver is loaded on the host.
///
/// `/sys` is filtered inside the flatpak and `/sys/module` does not exist
/// there, so in a sandbox we ask the host, the same way the server is started.
fn nvidia_driver_loaded() -> bool {
    if Path::new(FLATPAK_SPAWN).exists() {
        let probe = format!("[ -d {NVIDIA_MODULE_PATH} ]");
        Command::new(FLATPAK_SPAWN)
            .args(["--host", "sh", "-c", &probe])
            .status()
            .map(|status| status.success())
            // A failed probe is not worth reporting: the GUI still starts, it
            // just renders the way it did before.
            .unwrap_or(false)
    } else {
        Path::new(NVIDIA_MODULE_PATH).is_dir()
    }
}
