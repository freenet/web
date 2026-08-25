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

Found a bug, or want something Freenet does not do yet? Please tell us. This page points you at the
right place to say it.

Freenet is a network plus a set of separate apps built on top of it, and each one keeps its own list
of bugs. So the first step is picking which one you were using. If you pick wrong we will move your
report for you, so it is not worth agonising over.

{{< alert type="warning" >}} **Filing a report on GitHub needs a free GitHub account.** If you would
rather not create one, say it in [our Matrix room](https://matrix.to/#/#freenet-locutus:matrix.org)
instead and someone will file it for you. {{< /alert >}}

## What were you using?

### Group chat (River)

Messages not sending, a room that will not load, an invite that does not work, members missing,
anything in the chat interface itself.

→ [Report a River bug](https://github.com/freenet/river/issues/new) ·
[existing River issues](https://github.com/freenet/river/issues)

### Freenet itself

Install or uninstall trouble, the node will not start or will not stay running, no connections, high
CPU or bandwidth use, the menu bar or system tray icon, errors from the `freenet` command. Anything
underneath the apps rather than inside one.

→ [Report a Freenet bug](https://github.com/freenet/freenet-core/issues/new) ·
[existing Freenet issues](https://github.com/freenet/freenet-core/issues)

### This website

A broken link, documentation that is wrong or confusing, a page that will not load, a typo.

→ [Report a website bug](https://github.com/freenet/web/issues/new) ·
[existing website issues](https://github.com/freenet/web/issues)

### Ghost Keys

Anything about donations, Ghost Key certificates, or the vault.

→ [Report a Ghost Keys bug](https://github.com/freenet/ghostkeys/issues/new) ·
[existing Ghost Keys issues](https://github.com/freenet/ghostkeys/issues)

### One of the other apps

Delta, Mail, freenet-git, Atlas and the rest each keep their own list. Every app on the
[apps page](/apps/) links to its own project, and you will find its issues there.

### Not sure

Use the [Freenet issue tracker](https://github.com/freenet/freenet-core/issues/new). It is the one
the maintainers watch most closely, and they will move your report if it belongs somewhere else.
Describing clearly what happened matters far more than landing it in the right place first time.

If you would rather have someone work it out with you, ask in
[Matrix](https://matrix.to/#/#freenet-locutus:matrix.org).

## What to put in a bug report

You do not need to diagnose the problem. These are what make a report easy to act on:

- What you were doing, what you expected, and what happened instead.
- Whether it happens every time or only sometimes.
- Your operating system, and how you installed Freenet.
- A screenshot, if it is something you can see.

### The quick way to gather diagnostics

If Freenet is installed on your machine, run this in a terminal:

```bash
freenet service report
```

It asks you to describe the problem, gathers your version, operating system, recent log entries and
configuration, uploads them, and prints a short **report code** that looks like `X7K2M9`. Put that
code in your bug report and the maintainers can pull up everything they need.

That upload includes recent log lines and your Freenet configuration file. If you would rather see
what is in it first, `freenet service report --local report.json` writes it to a file on your own
machine instead of uploading it.

## Prefer to talk to a person?

Our [Matrix room](https://matrix.to/#/#freenet-locutus:matrix.org) is where Freenet's developers and
users are. It is a good place for "is this even a bug?", for questions the FAQ does not answer, and
for anything you would rather not write up as a formal report.

## Want to help fix things?

A clear bug report is already a real contribution. If you want to go further, every Freenet project
takes pull requests and each has a `CONTRIBUTING.md` explaining how, starting with
[freenet-core](https://github.com/freenet/freenet-core/blob/main/CONTRIBUTING.md). The
[Build section](/build/) covers how Freenet works and how to write apps for it.
