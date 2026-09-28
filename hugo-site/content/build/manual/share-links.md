---
title: "Share Links"
date: 2026-09-27
draft: false
---

Any Freenet app can be linked to from outside Freenet -- a chat message, an email, a QR code -- with
a link that works whether or not the person clicking it has Freenet installed. Every app uses the
same page, [freenet.org/open](/open/), instead of a per-app landing page.

---

## The format

```
https://freenet.org/open#<contract-id>/<path>?<query>#<app-fragment>
```

- `<contract-id>` is the base58 contract id from your app's own local URL
  (`http://127.0.0.1:7509/v1/contract/web/<contract-id>/...`).
- `<path>`, `<query>` and `<app-fragment>` are whatever already follows the contract id in your
  app's local link -- copy them as-is.

Everything after the _first_ `#` is a URL fragment. Browsers never send a fragment to the server, so
freenet.org never learns which contract, path or query a visitor is opening: it serves the same
static page to everyone and reads the rest with JavaScript, in the visitor's own browser. That is
also why an app's own fragment (state like `#store=...` or `#room=...`) needs a second, literal `#`
inside the outer one -- `/open` passes everything after the contract id through unchanged, it never
parses it apart.

## Example

A Harvest store with local link

```
http://127.0.0.1:7509/v1/contract/web/6FzSeAUKcqJrveKyU8RJgGKc5jRB1Z2juvxXtwTA4Em9/#store=Ab3Kx9Qm2Zp7Rt4L
```

becomes

```
https://freenet.org/open#6FzSeAUKcqJrveKyU8RJgGKc5jRB1Z2juvxXtwTA4Em9/#store=Ab3Kx9Qm2Zp7Rt4L
```

## What the page does

`/open` validates the contract id and shows four buttons, in this order:

- **Open on this computer** (currently the highlighted, primary button) -- the target on the
  visitor's own local peer (`http://127.0.0.1:7509/v1/contract/web/<contract-id>/...`), for anyone
  who already has Freenet installed and running.
- **Use in your browser** -- the same target on `try.freenet.org`, a peer we host, for anyone who
  wants to look without installing anything. Apps that hold keys a visitor should keep on their own
  peer, currently just the Ghost Key vault, get a note saying so instead of this button; the list is
  `LOCAL_ONLY` in `open-link.html`.
- **Open in Freenet** -- `freenet:<contract-id>/<path>...`, which the visitor's own Freenet opens
  on their local peer. The handler ([freenet-core#5726](https://github.com/freenet/freenet-core/issues/5726))
  ships in the first release after 0.2.139, so until peers have updated this button is styled and
  ordered as a secondary option. The link has no `//`: in `freenet://<contract-id>` the
  case-sensitive contract id would be the URL's host, which some desktops lowercase before the
  handler sees it. (The handler accepts both forms.)
- **Get Freenet** -- the [install guide](/quickstart/).

An invalid or truncated fragment shows a "this link looks broken" message instead of guessing at
one.

## What it doesn't do

`/open` makes no network request of its own, and it doesn't try to detect whether Freenet is running
on the visitor's computer. Checking that would mean probing `localhost`, which leaks whether Freenet
is installed to anything embedding the page and, on some browsers, triggers a local-network access
prompt. Whoever built the app already knows how to turn a link back into a working session; `/open`
only carries the address there unread.
