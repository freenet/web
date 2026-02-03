---
title: "Quick Start"
date: 2025-01-01
draft: false
---

{{< alert type="warning" >}}
**Alpha Software:** Freenet is under active development and may be unstable. New versions are released frequently—sometimes multiple times per day—and older versions will stop working as the network evolves.

During alpha testing:
- **Telemetry:** Your peer will report diagnostic data to our servers for debugging purposes, including peer activity and general system info (e.g., your OS).
- **Auto-updates:** Your peer may automatically update when new versions become available.
{{< /alert >}}

Get started with Freenet in minutes. Install the software and join River—the world's first truly decentralized group chat.

## Step 1: Install Freenet

### Linux & macOS

Run this command in your terminal:

```bash
curl -fsSL https://freenet.org/install.sh | sh
```

This downloads and installs Freenet, then starts it as a background service.

### Windows

Install from source with [Cargo](https://rustup.rs/):

```bash
cargo install freenet
```

Then start Freenet with `freenet network`.

### Docker

Freenet provides official Docker images. Pull and run with:

```bash
docker run -p 50509:50509 -p 56208:56208/udp freenetorg/freenet:latest
```

**Windows (WSL2) users:** Docker Desktop on Windows requires additional configuration for UDP networking. See the [Windows WSL2 Setup](#windows-wsl2-docker-setup) section in Troubleshooting below.

## Step 2: Join Freenet Official

Get an invite to our community chat. You can request up to 5 invites per day.

{{< river-invite-button room="Freenet Official" >}}

Clicking the link will open River in your browser and automatically join you to the room using the invite code.

## Troubleshooting

If you run into problems, join our [Matrix chat](https://matrix.to/#/#freenet-locutus:matrix.org) for help.

**Network requirements:** Freenet uses UDP hole punching for peer-to-peer connections. Most home routers support this without configuration. Strict corporate firewalls may block connections.

### Windows WSL2 Docker Setup

Docker Desktop on Windows uses WSL2, which has networking layers that can block Freenet's UDP peer-to-peer connections. If River won't load or gets stuck at "Subscribing to room", follow these steps:

**1. Configure WSL2 networking**

Create or edit `%USERPROFILE%\.wslconfig` with:

```ini
[wsl2]
networkingMode=Mirrored
firewall=false

[experimental]
hostAddressLoopback=true
```

**2. Allow UDP through the Hyper-V firewall**

Open PowerShell as Administrator and run:

```powershell
Set-NetFirewallHyperVVMSetting -Name '{40E0AC32-46A5-438A-A0B2-2B479E8F2E90}' -DefaultInboundAction Allow
```

**3. Restart WSL**

```powershell
wsl --shutdown
```

Then restart Docker Desktop.

## What's Next?

- [Live Network Dashboard](http://nova.locut.us:3133/) - Watch real-time activity on the network
- [User Manual](/resources/manual/) - Learn how Freenet works
- [Video Talks](/resources/video-talks/) - Watch presentations about Freenet
- [FAQ](/faq/) - Common questions and answers
- [Get Involved](/community/get-involved/) - Contribute to the project
