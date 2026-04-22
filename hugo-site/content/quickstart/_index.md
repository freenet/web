---
title: "Quick Start"
date: 2025-01-01
draft: false
---

{{< alert type="warning" >}}
**Alpha Software:** Freenet is under active development and may be unstable. New versions are released frequently (sometimes multiple times per day), and older versions will stop working as the network evolves.

During alpha testing:
- **Telemetry:** Your peer will report diagnostic data to our servers for debugging purposes, including peer activity and general system info (e.g., your OS).
- **Auto-updates:** Your peer will automatically update when new versions become available.
{{< /alert >}}

Get started with Freenet in minutes. Install the software and join River, the world's first truly decentralized group chat.

## Step 1: Install Freenet

{{< os-install >}}

## Step 2: Chat with other Freenet users on River

River is our decentralized group chat, built on Freenet. Click below to get an invite to the
**Freenet Official** room (up to 5 invites per day).

{{< river-invite-button room="Freenet Official" >}}

Clicking the link will open River in your browser and automatically join you to the room using the invite
code.

### CLI Alternative: riverctl

River also has a full-featured command-line interface. If you have Rust tooling installed, you can install it
with:

```bash
cargo install riverctl
```

To accept an invite via the CLI, click "Get Invite Code" above, expand "Using riverctl? Copy invite code" to
copy the code, then run:

```bash
riverctl invite accept <invite-code>
```

Once joined, you can send and receive messages entirely from the terminal:

```bash
riverctl message send <room-owner-key> "Hello from the CLI!"
riverctl message stream <room-owner-key>   # live message stream
riverctl room list                          # list your rooms
```

Run `riverctl --help` for the full list of commands.

## Uninstalling

To remove Freenet completely:

```bash
freenet uninstall                                 # preferred
curl -fsSL https://freenet.org/uninstall.sh | sh  # fallback
```

Either command stops the service, removes the binaries, and (with confirmation) deletes your data, config, cache, and logs. Pass `--purge` to skip the confirmation, or `--keep-data` to preserve all of them. The second form is useful when the installed `freenet` binary is missing, broken, or not on your PATH.

**Do not run `sudo freenet uninstall`** for a normal `curl | sh` install. The installer puts the binary in `~/.local/bin`, which is not on `sudo`'s default PATH, so `sudo freenet uninstall` fails with `command not found` and your install is left untouched. Only use `sudo` if you originally installed with `--system` (in which case the unit file is at `/etc/systemd/system/freenet.service`).

If `freenet` isn't on your PATH, call it by full path: `~/.local/bin/freenet uninstall`.

**Installed with `cargo install freenet`?** The binary lives in `~/.cargo/bin/freenet`. Run `cargo uninstall freenet` (and `cargo uninstall fdev` if you also installed that), then remove the data directories listed below.

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

**macOS (DMG install).** Click the menu bar rabbit → **Quit Freenet**, then drag `Freenet.app` from `/Applications` to the Trash. To also remove data and configuration:

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

**macOS (legacy `install.sh` install).** Older installs use the `org.freenet.node` agent and binaries under `~/.local/bin`:

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

**Windows.** The PowerShell installer (`irm https://freenet.org/install.ps1 | iex`) installs the binaries under `%LOCALAPPDATA%\Freenet\bin\`, and `freenet uninstall` has a known gap: it removes the data directory but may leave the config folder behind. After running the uninstall, delete any remaining folders manually (PowerShell):

```powershell
# Binaries
Remove-Item -Recurse -Force "$env:LOCALAPPDATA\Freenet\bin" -ErrorAction SilentlyContinue

# Data and logs (Local AppData)
Remove-Item -Recurse -Force "$env:LOCALAPPDATA\The Freenet Project Inc\Freenet" -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force "$env:LOCALAPPDATA\freenet\logs" -ErrorAction SilentlyContinue

# Config (Roaming AppData)
Remove-Item -Recurse -Force "$env:APPDATA\The Freenet Project Inc\Freenet" -ErrorAction SilentlyContinue
```

Also check `HKCU:\Software\Microsoft\Windows\CurrentVersion\Run` in the registry for any leftover `Freenet` startup entry and remove it.

## Troubleshooting

If you run into problems, join our [Matrix chat](https://matrix.to/#/#freenet-locutus:matrix.org) for help.

**Invite didn't work?** If River opened but you're not in the room, try restarting Freenet (`freenet service restart`), then come back to this page and click the invite button again for a fresh invite code. If you see the room but can't send messages, click the **"i"** icon next to the room name, click **"Leave Room"**, then get a new invite.

**Containers & headless servers:** If service installation fails (common in LXC/Docker), use the system-wide
service instead: `sudo freenet service install --system`

**Network requirements:** Freenet uses UDP hole punching for peer-to-peer connections. Most home routers support this without configuration. Strict corporate firewalls may block connections.

## What's Next?

- [Live Network Dashboard](http://nova.locut.us:3133/) - Watch real-time activity on the network
- [User Manual](/build/manual/) - Learn how Freenet works
- [Video Talks](/about/video-talks/) - Watch presentations about Freenet
- [FAQ](/about/faq/) - Common questions and answers
- [Get Involved](/community/get-involved/) - Contribute to the project
