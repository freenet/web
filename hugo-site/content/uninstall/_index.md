---
title: "Uninstalling Freenet"
date: 2026-04-22
draft: false
---

Pick the install method you used. If you're not sure, the **DMG / installer** sections cover the most common case on macOS and Windows.

## macOS (DMG install)

Click the menu bar rabbit, choose **Quit Freenet**, and drag `Freenet.app` from `/Applications` to the Trash. To also remove data and configuration:

```bash
# Remove the launch-at-login agent
launchctl bootout gui/$UID/org.freenet.Freenet 2>/dev/null
rm -f ~/Library/LaunchAgents/org.freenet.Freenet.plist

# Remove data, config, cache, and logs
rm -rf ~/Library/Application\ Support/The-Freenet-Project-Inc.Freenet \
       ~/Library/Caches/The-Freenet-Project-Inc.Freenet \
       ~/Library/Caches/Freenet \
       ~/Library/Logs/freenet
```

## macOS (legacy `install.sh` install)

Older installs use the `org.freenet.node` agent and binaries under `~/.local/bin`:

```bash
# Stop and remove the legacy user agent
launchctl unload ~/Library/LaunchAgents/org.freenet.node.plist 2>/dev/null
rm -f ~/Library/LaunchAgents/org.freenet.node.plist

# Remove binaries
rm -f ~/.local/bin/freenet ~/.local/bin/fdev \
      ~/.local/bin/freenet-service-wrapper.sh

# Remove data, config, cache, and logs
rm -rf ~/Library/Application\ Support/The-Freenet-Project-Inc.Freenet \
       ~/Library/Caches/The-Freenet-Project-Inc.Freenet \
       ~/Library/Caches/The-Freenet-Project-Inc.freenet \
       ~/Library/Logs/freenet
```

## Windows

The PowerShell installer (`irm https://freenet.org/install.ps1 | iex`) places binaries under `%LOCALAPPDATA%\Freenet\bin\`. `freenet uninstall` removes the data directory but may leave the config folder behind, so do a manual pass afterward (PowerShell):

```powershell
# Binaries
Remove-Item -Recurse -Force "$env:LOCALAPPDATA\Freenet\bin" -ErrorAction SilentlyContinue

# Data and logs (Local AppData)
Remove-Item -Recurse -Force "$env:LOCALAPPDATA\The Freenet Project Inc\Freenet" -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force "$env:LOCALAPPDATA\freenet\logs" -ErrorAction SilentlyContinue

# Config (Roaming AppData)
Remove-Item -Recurse -Force "$env:APPDATA\The Freenet Project Inc\Freenet" -ErrorAction SilentlyContinue
```

Also check `HKCU:\Software\Microsoft\Windows\CurrentVersion\Run` in the registry for a leftover `Freenet` startup entry and remove it.

## Linux

```bash
freenet uninstall                                 # preferred
curl -fsSL https://freenet.org/uninstall.sh | sh  # fallback
```

Either command stops the service, removes the binaries, and (with confirmation) deletes your data, config, cache, and logs. Pass `--purge` to skip the confirmation, or `--keep-data` to preserve those files. The second form is useful when the installed `freenet` binary is missing, broken, or not on your PATH.

**Do not run `sudo freenet uninstall`** for a normal `curl | sh` install. The installer puts the binary in `~/.local/bin`, which is not on `sudo`'s default PATH, so `sudo freenet uninstall` fails with `command not found` and your install is left untouched. Only use `sudo` if you originally installed with `--system` (the unit file lives at `/etc/systemd/system/freenet.service`).

If `freenet` isn't on your PATH, call it by full path: `~/.local/bin/freenet uninstall`.

**Installed with `cargo install freenet`?** The binary lives in `~/.cargo/bin/freenet`. Run `cargo uninstall freenet` (and `cargo uninstall fdev` if you also installed that), then remove the data directories below.

### Manual fallback

If the binary is missing or broken, remove everything by hand:

```bash
# Stop and remove the user service (if installed)
systemctl --user disable --now freenet.service 2>/dev/null
rm -f ~/.config/systemd/user/freenet.service

# Remove binaries
rm -f ~/.local/bin/freenet ~/.local/bin/fdev

# Remove data, config, cache, and logs
rm -rf ~/.local/share/freenet ~/.config/freenet \
       ~/.cache/freenet ~/.local/state/freenet
```
