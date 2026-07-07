---
title: "Try Freenet: Join River"
date: 2025-01-01
draft: false
---

Install Freenet and you'll join the live chat room where the project's developers and users talk.
It's the fastest way to see the network in action.

The room runs as a Freenet contract: code that lives across the peer-to-peer network instead of on
a central server. No company hosts it, no account to make, no admin who can shut it down.

{{< alert type="warning" >}} **Alpha notes:** Freenet is under active development.

- **Auto-updates:** your peer updates as the network evolves; older versions stop working over
  time.
- **Telemetry:** your peer reports diagnostic data to help debug the network, including peer
  activity and general system info such as your OS.
- Do not use alpha builds for anything sensitive yet. {{< /alert >}}

## Step 1: Install Freenet to enter the room

First install the app. It runs a local peer in the background, then opens apps like River in your
browser.

{{< os-install >}}

## Step 2: Join the room

Click below to get an invite to the **Freenet Official** room. Invites are limited to 20 per day.

{{< river-invite-button room="Freenet Official" >}}

Clicking the link opens River in your browser and automatically joins you to the room using the
invite code.

Prefer the terminal? River has a full-featured CLI, `riverctl`. See the
[riverctl README](https://github.com/freenet/river/blob/main/cli/README.md) for install and usage.

## Troubleshooting

If you run into problems, join our [Matrix chat](https://matrix.to/#/#freenet-locutus:matrix.org)
for help.

**Invite didn't work?** If River opened but you're not in the room, try restarting Freenet
(`freenet service restart`), then come back to this page and click the invite button again for a
fresh invite code. If you see the room but can't send messages, click the **"i"** icon next to the
room name, click **"Leave Room"**, then get a new invite.

**Containers & headless servers:** If service installation fails (common in LXC/Docker), use the
system-wide service instead: `sudo freenet service install --system`

**Network requirements:** Freenet uses UDP hole punching for peer-to-peer connections. Most home
routers support this without configuration. Strict corporate firewalls may block connections.

Need to remove Freenet? See the [uninstall guide](/uninstall/).

## What's Next?

- [Live Network Dashboard](http://nova.locut.us:3133/) - Watch real-time activity on the network
- [User Manual](/build/manual/) - Learn how Freenet works
- [Video Talks](/about/video-talks/) - Watch presentations about Freenet
- [FAQ](/about/faq/) - Common questions and answers
- [Get Involved](/community/get-involved/) - Contribute to the project
