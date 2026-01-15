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

**Supported platforms:** Linux, macOS

Run this command in your terminal:

```bash
curl -fsSL https://freenet.org/install.sh | sh
```

This downloads and installs Freenet, then starts it as a background service. Your browser will open to [River](http://127.0.0.1:7509/v1/contract/web/raAqMhMG7KUpXBU2SxgCQ3Vh4PYjttxdSWd9ftV7RLv/) once the peer is ready.

**Windows:** The install script doesn't currently support Windows, but Freenet runs on Windows and can be installed from source with [Cargo](https://rustup.rs/):

```bash
cargo install freenet
```

Then start Freenet with `freenet run`.

## Step 2: Join Freenet Official

Get an invite to our community chat. You can request up to 5 invites per day.

{{< river-invite-button room="Freenet Official" >}}

Clicking the link will open River in your browser and automatically join you to the room using the invite code.

## Troubleshooting

If you run into problems, join our [Matrix chat](https://matrix.to/#/#freenet-locutus:matrix.org) for help.

**Network requirements:** Freenet uses UDP hole punching for peer-to-peer connections. Most home routers support this without configuration. Strict corporate firewalls may block connections.

## What's Next?

- [Live Network Dashboard](http://nova.locut.us:3133/) - Watch real-time activity on the network
- [User Manual](/resources/manual/) - Learn how Freenet works
- [Video Talks](/resources/video-talks/) - Watch presentations about Freenet
- [FAQ](/faq/) - Common questions and answers
- [Get Involved](/community/get-involved/) - Contribute to the project
