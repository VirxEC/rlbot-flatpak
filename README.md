# RLBot v5 flatpak

Linux packaging for RLBot v5. This repository builds `org.rlbot.gui`, a flatpak that runs [rlbotgui](https://github.com/RLBot/gui) on Linux. The flatpak keeps [rlbotgui](https://github.com/RLBot/gui) and [RLBotServer](https://github.com/RLBot/core) up to date from their GitHub releases.

The server runs on the host so it can see the game and spawn bots. The GUI runs inside the sandbox.

## Installing

### With updates (recommended)

Add the repository once, then install. The repository is GPG-signed and served from GitHub Pages. `flatpak update` detects new launcher releases automatically:

    flatpak remote-add --user --if-not-exists rlbot-flatpak https://raw.githubusercontent.com/VirxEC/rlbot-flatpak/master/org.rlbot.gui.flatpakrepo
    flatpak install --user rlbot-flatpak org.rlbot.gui

Update later with:

    flatpak update --user org.rlbot.gui

### Single file (no auto-updates)

Download `org.rlbot.gui.flatpak` from the latest [release](https://github.com/VirxEC/rlbot-flatpak/releases) and install it directly. No remote is attached. Re-download the file for every new release:

    flatpak install ./org.rlbot.gui.flatpak

### First run

The first launch downloads RLBotServer and rlbotgui from their GitHub releases (~28 MB). It downloads them into `~/.var/app/org.rlbot.gui/data/RLBot5/bin/`. It starts RLBotServer on the host. It opens the GUI.

### Logs

The launcher and RLBotServer output stream to your terminal when you run `flatpak run org.rlbot.gui`. The output is written to `~/.var/app/org.rlbot.gui/data/RLBot5/logs/` (`rlbot.log` and `rlbotserver.log`). Each run creates fresh files, like the Windows launcher's console.

When you launch the app from the app grid, the app runs in your system's terminal. The window closes when you close the GUI. On desktops that do not launch `Terminal=true` apps in a terminal, the launcher opens a terminal itself.

## How updates work

The flatpak itself only contains the launcher and the runtime libraries (WebKitGTK 4.1 / GTK3 from `org.gnome.Platform`). On every launch, the launcher:

1. Fetches the latest releases of `RLBot/core` and `RLBot/gui` in parallel. It downloads whichever binary changed. It verifies the download with sha256 against the release asset digest. It always checks both, so the pair stays up to date together.
2. Starts `RLBotServer` on the host via `flatpak-spawn --host`. It must run outside the sandbox to see the game in `/proc` and spawn Steam/Proton and bots.
3. Runs `rlbotgui` inside the sandbox, where it gets WebKitGTK from the runtime.

Binaries are cached in `~/.var/app/org.rlbot.gui/data/RLBot5/bin/`. This is a real host path. The host-side server can execute from the same directory that the GUI uses for botpacks.

## Building locally

This repository contains the `rlbot-launcher` source and the `org.rlbot.gui.yml` manifest that bundles it (plus zenity, which backs the GUI's file pickers).

### Prerequisites (Debian/Ubuntu)

    sudo apt install flatpak flatpak-builder
    flatpak remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo

Install the GNOME runtime and SDK that the manifest pins (`runtime-version`), plus the Rust toolchain extension. The GNOME SDK does not ship cargo:

    flatpak install flathub org.gnome.Platform//50 org.gnome.Sdk//50
    flatpak install flathub org.freedesktop.Sdk.Extension.rust-stable//25.08

`org.gnome.Platform//50` is based on freedesktop-sdk 25.08. This is why the rust-stable extension is on the `25.08` branch even though the GNOME runtime is version 50. Bump both together with the manifest if you move to a newer GNOME runtime.

### Build

    flatpak-builder --force-clean --disable-rofiles-fuse --repo=repo build-dir org.rlbot.gui.yml

Notes:

- The first build needs network. It downloads the crates.io dependencies and the zenity source from gitlab.gnome.org.
- The launcher module uses `build-args: [--share=network]`. The flatpak-builder shipped with Ubuntu 24.04 (1.4.x) ignores the newer `network: true` build-option. You will see a harmless "Unknown property network" warning from that version. Newer flatpak-builder versions honor `network: true` directly, so both are kept in the manifest.
- `--disable-rofiles-fuse` avoids a dependency on rofiles-fuse when exporting to the local `repo/` directory.
- Build artifacts (`build-dir/`, `repo/`, `.flatpak-builder/`, `target/`) are gitignored.

### Install and run

Install what you just built into your user installation (no root needed):

    flatpak-builder --force-clean --user --install build-dir org.rlbot.gui.yml
    flatpak run org.rlbot.gui

`--user` targets your user installation. Without it, `--install` tries the system installation and fails with "Flatpak system operation ConfigureRemote not allowed for user" unless you run it as root.

Or create a distributable bundle (annotated with the flathub runtime-repo, so it installs standalone):

    flatpak build-bundle --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo repo org.rlbot.gui.flatpak org.rlbot.gui
    flatpak install ./org.rlbot.gui.flatpak

## Publishing

The flatpak repository is served to users from the `gh-pages` branch via GitHub Pages. Tag a release (e.g. `v0.1.0`). The `Build flatpak` workflow then:

1. Builds `org.rlbot.gui.flatpak` and attaches it to the release.
2. Exports the ostree repository to the `gh-pages` branch. This is the remote that the "Installing with updates" command above points at.

The repository is GPG-signed. The `Build flatpak` workflow signs the exported commits and summary using the key stored in the repository secrets (`GPG_PRIVATE_KEY`, `GPG_PASSPHRASE`, `GPG_KEY_ID`, `GPG_KEY_GRIP`). Flatpak verifies pulls against the public key embedded in `org.rlbot.gui.flatpakrepo`. Anyone with write access to the repository still controls what gets served. Keeping the signing key and repository secrets safe is therefore critical.

The GUI and server update themselves from GitHub on every launch. The flatpak needs a new release only when the launcher changes.

## Developing the launcher

    cargo run -- --online

Run the command outside a flatpak. It downloads to `~/.local/share/RLBot5/bin` and spawns the server directly instead of via `flatpak-spawn`. Flags mirror the Windows launcher: `--force-update-gui`, `--force-update-server`, `--offline`, `--online`.
