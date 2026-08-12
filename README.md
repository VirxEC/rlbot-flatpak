# RLBot v5 flatpak

Linux packaging for RLBot v5: this repo builds `org.rlbot.gui`, a flatpak that
runs [rlbotgui](https://github.com/RLBot/gui) on Linux and keeps it and
[RLBotServer](https://github.com/RLBot/core) up to date from their GitHub
releases. The server runs on the host so it can see the game and spawn bots;
the GUI runs inside the sandbox.

## Installing

### With updates (recommended)

Add the repo once, then install — the repo is served from GitHub Pages, so
`flatpak update` picks up new launcher releases automatically:

    flatpak remote-add --user --if-not-exists --no-gpg-verify rlbot-flatpak https://virxec.github.io/rlbot-flatpak/
    flatpak install --user rlbot-flatpak org.rlbot.gui

Update later with:

    flatpak update --user org.rlbot.gui

`--no-gpg-verify` is needed because the repo is not GPG-signed; see
[Publishing](#publishing) for the trade-off.

### Single file (no auto-updates)

Download `org.rlbot.gui.flatpak` from the latest
[release](https://github.com/VirxEC/rlbot-flatpak/releases) and install it
directly. No remote is attached, so re-download it for every new release:

    flatpak install ./org.rlbot.gui.flatpak

### First run

Either way, the first launch downloads RLBotServer and rlbotgui from their
GitHub releases (~28 MB) into `~/.var/app/org.rlbot.gui/data/RLBot5/bin/`,
starts RLBotServer on the host, and opens the GUI.

### Logs

Launcher and RLBotServer output is streamed to your terminal when you run
`flatpak run org.rlbot.gui`, and written to
`~/.var/app/org.rlbot.gui/data/RLBot5/logs/` (`rlbot.log` and
`rlbotserver.log`), fresh for each run like the Windows launcher's console.
Launched from the app grid, the app runs in your system's terminal and the
window closes when you close the GUI; on desktops that don't launch
`Terminal=true` apps in a terminal, the launcher opens one itself.

## How updates work

The flatpak itself only contains the launcher and the runtime libraries
(WebKitGTK 4.1 / GTK3 from `org.gnome.Platform`). On every launch the launcher:

1. Fetches the latest releases of `RLBot/core` **and** `RLBot/gui` in parallel
   and downloads whichever binary changed, sha256-verified against the release
   asset digest. Checking one always checks the other, so the pair stays in
   sync.
2. Starts `RLBotServer` **on the host** via `flatpak-spawn --host`. It must run
   outside the sandbox to see the game in `/proc` and spawn Steam/Proton and
   bots.
3. Runs `rlbotgui` inside the sandbox, where it gets WebKitGTK from the
   runtime.

Binaries are cached in `~/.var/app/org.rlbot.gui/data/RLBot5/bin/`, a real
host path, so the host-side server can execute from the same directory the GUI
uses for botpacks.

## Building locally

This repo contains the `rlbot-launcher` source and the `org.rlbot.gui.yml`
manifest that bundles it (plus zenity, which backs the GUI's file pickers).

### Prerequisites (Debian/Ubuntu)

    sudo apt install flatpak flatpak-builder
    flatpak remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo

Install the GNOME runtime and SDK that the manifest pins (`runtime-version`),
plus the Rust toolchain extension — the GNOME SDK does not ship cargo:

    flatpak install flathub org.gnome.Platform//50 org.gnome.Sdk//50
    flatpak install flathub org.freedesktop.Sdk.Extension.rust-stable//25.08

`org.gnome.Platform//50` is based on freedesktop-sdk 25.08, which is why the
rust-stable extension is on the `25.08` branch even though the GNOME runtime is
version 50. Bump both together with the manifest if you move to a newer GNOME
runtime.

### Build

    flatpak-builder --force-clean --disable-rofiles-fuse --repo=repo build-dir org.rlbot.gui.yml

Notes:

- The first build needs network: it downloads the crates.io dependencies and
  the zenity source from gitlab.gnome.org.
- The launcher module uses `build-args: [--share=network]` because the
  flatpak-builder shipped with Ubuntu 24.04 (1.4.x) ignores the newer
  `network: true` build-option (you'll see a harmless "Unknown property
  network" warning from that version). Newer flatpak-builder versions honor
  `network: true` directly, so both are kept in the manifest.
- `--disable-rofiles-fuse` avoids a dependency on rofiles-fuse when exporting
  to the local `repo/` directory.
- Build artifacts (`build-dir/`, `repo/`, `.flatpak-builder/`, `target/`) are
  gitignored.

### Install and run

Install what you just built into your user installation (no root needed):

    flatpak-builder --force-clean --user --install build-dir org.rlbot.gui.yml
    flatpak run org.rlbot.gui

`--user` targets your user installation; without it `--install` tries the
system installation and fails with "Flatpak system operation ConfigureRemote
not allowed for user" unless run as root.

Or create a distributable bundle (annotated with the flathub runtime-repo, so
it installs standalone):

    flatpak build-bundle --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo repo org.rlbot.gui.flatpak org.rlbot.gui
    flatpak install ./org.rlbot.gui.flatpak

## Publishing

Source lives at https://github.com/VirxEC/rlbot-flatpak. First push:

    git branch -M main
    git remote add origin https://github.com/VirxEC/rlbot-flatpak.git
    git push -u origin main

Then enable GitHub Pages once: Settings → Pages → Source: *Deploy from a
branch* → Branch: `gh-pages` (root). The workflow creates that branch on the
first release.

Tag a release (e.g. `v0.1.0`); the `Build flatpak` workflow then:

1. Builds `org.rlbot.gui.flatpak` and attaches it to the release.
2. Exports the ostree repo to the `gh-pages` branch — this is the remote the
   "Installing with updates" command above points at.

The repo is unsigned, so users add the remote with `--no-gpg-verify`; anyone
who can push to `gh-pages` could replace the launcher. The launcher verifies
the downloaded RLBotServer/rlbotgui binaries against GitHub's release
sha256 digests, but hardening the repo itself (a GPG signing key in the
workflow, published in a `.flatpakrepo` file) is a possible follow-up.

The GUI and server update themselves from GitHub on every launch, so the
flatpak only needs re-releasing when the launcher changes.

## Developing the launcher

    cargo run -- --online

Runs outside a flatpak: downloads to `~/.local/share/RLBot5/bin` and spawns
the server directly instead of via `flatpak-spawn`. Flags mirror the Windows
launcher: `--force-update-gui`, `--force-update-server`, `--offline`,
`--online`.

Note: `cargo build` generates `Cargo.lock` on first run — commit it for
reproducible builds.
