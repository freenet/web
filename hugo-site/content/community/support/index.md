---
title: "Report a Bug or Get Help"
description:
  "Where to report a Freenet bug or ask for a feature: which project, which issue tracker, and what
  to include."
date: 2026-08-25
draft: false
aliases:
  - /report/
  - /community/get-involved/
---

Found a bug, or want something Freenet doesn't do yet? Please tell us. This page points you at the
right place to say it.

Freenet is a network plus a set of separate apps built on top of it, and each one keeps its own list
of bugs. So the first step is picking which one you were using. If you pick wrong, we'll move your
report for you.

Filing a report on GitHub needs a free GitHub account. If you'd rather not create one, say it in
[our Matrix room](https://matrix.to/#/#freenet-locutus:matrix.org) instead and someone will file it
for you.

## What were you using?

### Group chat (River)

Messages not sending, a room that won't load, an invite that doesn't work, members missing, anything
in the chat interface itself.

→ [Report a River bug](https://github.com/freenet/river/issues/new) ·
[existing River issues](https://github.com/freenet/river/issues)

### Freenet itself

Install or uninstall trouble, the node won't start or won't stay running, no connections, high CPU
or bandwidth use, the menu bar or system tray icon, errors from the `freenet` command. Anything
underneath the apps rather than inside one.

→ [Report a Freenet bug](https://github.com/freenet/freenet-core/issues/new) ·
[existing Freenet issues](https://github.com/freenet/freenet-core/issues)

### This website

A broken link, documentation that's wrong or confusing, a page that won't load, a typo.

→ [Report a website bug](https://github.com/freenet/web/issues/new) ·
[existing website issues](https://github.com/freenet/web/issues)

### Ghost Keys

Anything about donations, Ghost Key certificates, or the Ghostkey Vault.

→ [Report a Ghost Keys bug](https://github.com/freenet/ghostkeys/issues/new) ·
[existing Ghost Keys issues](https://github.com/freenet/ghostkeys/issues)

### One of the other apps

Each keeps its own list: [Delta](https://github.com/freenet/delta/issues/new),
[Mail](https://github.com/freenet/mail/issues/new),
[freenet-git](https://github.com/freenet/freenet-git/issues/new),
[Raven](https://github.com/freenet/raven/issues/new),
[Atlas](https://github.com/freenet/atlas/issues/new) and
[Harvest](https://github.com/freenet/harvest/issues/new). The [apps page](/apps/) says what each one
does, if you're not sure which you were using.

### Not sure

Use the [Freenet issue tracker](https://github.com/freenet/freenet-core/issues/new). It's the one
the maintainers watch most closely, and they'll move your report if it belongs somewhere else.
Describing clearly what happened matters far more than landing it in the right place first time.

If you'd rather have someone work it out with you, ask in
[Matrix](https://matrix.to/#/#freenet-locutus:matrix.org).

## What to put in a bug report

You don't need to diagnose the problem. These are what make a report easy to act on:

- What you were doing, what you expected, and what happened instead.
- Whether it happens every time or only sometimes.
- Your operating system, and how you installed Freenet.
- A screenshot, if it's something you can see.

### The quick way to gather diagnostics

If Freenet is installed on your machine, run this in a terminal:

```bash
freenet service report
```

It gathers your version, operating system, recent log entries and configuration, then asks you to
describe the problem, uploads everything, and prints a short **report code** that looks like
`X7K2M9`. Put that code in your bug report and the maintainers can pull up everything they need. The
step that queries your running node can take up to a minute, so give it a moment before the prompt
appears.

That upload is more than logs, so it's worth knowing what's in it: your machine's hostname, recent
log lines, your Freenet configuration file (which includes paths containing your username), and, if
your node is running, its current network state. That state covers your peer ID, the addresses of
the peers you're connected to, and the contracts you're subscribed to, which for River means the
rooms you're in. Your IP address is recorded when the upload is received.

If you'd rather read all of that before sending it, `freenet service report --local report.json`
writes the same bundle to a file on your own machine and uploads nothing. There's no way to send a
saved file afterwards, so run the command again without `--local` once you're happy with it.

If you'd rather not attach the code to a public issue, send it to us in
[Matrix](https://matrix.to/#/#freenet-locutus:matrix.org) instead and mention your issue number.

## Prefer to talk to a person?

Our [Matrix room](https://matrix.to/#/#freenet-locutus:matrix.org) is where Freenet's developers and
users are. It's a good place for "is this even a bug?", for questions the FAQ doesn't answer, and
for anything you'd rather not write up as a formal report.

## Want to help fix things?

A clear bug report is already a real contribution. If you want to go further,
[freenet-core's contributing guide](https://github.com/freenet/freenet-core/blob/main/CONTRIBUTING.md)
explains how the project takes changes. Read it before you write any code: feature work needs an
agreed issue first, and pull requests that skip that step get closed. The [Build section](/build/)
covers how Freenet works and how to write apps for it.
