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

## Step 2: Join Freenet Official

Get an invite to our community chat. You can request up to 5 invites per day.

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

## Troubleshooting

If you run into problems, join our [Matrix chat](https://matrix.to/#/#freenet-locutus:matrix.org) for help.

**Invite didn't work?** If River opened but you're not in the room, try restarting Freenet (`freenet service restart`), then come back to this page and click the invite button again for a fresh invite code. If you see the room but can't send messages, click the **"i"** icon next to the room name, click **"Leave Room"**, then get a new invite.

**Containers & headless servers:** If service installation fails (common in LXC/Docker), use the system-wide
service instead: `sudo freenet service install --system`

**Network requirements:** Freenet uses UDP hole punching for peer-to-peer connections. Most home routers support this without configuration. Strict corporate firewalls may block connections.

## What's Next?

- [Live Network Dashboard](http://nova.locut.us:3133/) - Watch real-time activity on the network
- [User Manual](/resources/manual/) - Learn how Freenet works
- [Video Talks](/resources/video-talks/) - Watch presentations about Freenet
- [FAQ](/faq/) - Common questions and answers
- [Get Involved](/community/get-involved/) - Contribute to the project
