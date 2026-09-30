---
title: "Share Links"
date: 2026-09-27
draft: false
---

Any Freenet app can be linked to from outside Freenet -- a chat message, an email, a QR code -- with
a link that works whether or not the person clicking it has Freenet installed. Every app uses the
same page, [freenet.org/open](/open/), instead of a per-app landing page.

Don't want to build the URL by hand? Visit [freenet.org/open](/open/) with nothing after it (no `#`)
and it doubles as a small tool: paste any Freenet link -- your app's own local link, a `freenet:` or
`freenet://` link, or an existing freenet.org/open link -- pick who it's for, and copy the result.
It runs entirely in your browser; nothing you paste is sent anywhere.

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

`/open` validates the contract id, then shows ONE big button plus a collapsed "Other ways to open"
with the rest -- a first-time visitor from an unfamiliar link should see exactly one obvious thing
to click, not a row of equally-weighted unknowns. The four possible buttons are:

- **Open on this computer** (the default primary button) -- the target on the visitor's own local
  peer (`http://127.0.0.1:7509/v1/contract/web/<contract-id>/...`), for anyone who already has
  Freenet installed and running.
- **Use in your browser** -- the same target on `try.freenet.org`, a peer we host, for anyone who
  wants to look without installing anything. It's a single shared demo peer, so it can be slow or
  busy under load -- see the `?via=browser` section below for the tradeoff this implies for
  higher-traffic sharing.
- **Open in Freenet** -- `freenet:<contract-id>/<path>...`, which the visitor's own Freenet opens on
  their local peer. The handler
  ([freenet-core#5726](https://github.com/freenet/freenet-core/issues/5726)) ships in the first
  release after 0.2.139, so until peers have updated this button stays in "Other ways to open"
  rather than being the default primary. The link has no `//`: in `freenet://<contract-id>` the
  case-sensitive contract id would be the URL's host, which some desktops lowercase before the
  handler sees it. (The handler accepts both forms.)
- **Get Freenet** -- the [install guide](/quickstart/).

An invalid or truncated fragment shows a "this link looks broken" message instead of guessing at
one. Visiting `/open/` with no fragment at all shows the link-making tool described above instead of
a dead end.

## Choosing the primary button: `?via=`

By default the primary button is **Open on this computer**, on the assumption that most people
following a link already have Freenet. If your audience mostly doesn't -- a link posted to Facebook
or a general audience, say -- add `?via=` before the `#`:

```
https://freenet.org/open?via=browser#<contract-id>/<path>?<query>#<app-fragment>
```

- **`?via=browser`** -- skips the page entirely. Once the link validates, the page immediately
  redirects to the `try.freenet.org` target with no click needed, since that destination is a plain
  `https://` page and always reachable, so no separate landing page is needed to explain it. Plain
  https is not the `freenet:` scheme, so this redirect doesn't need a user gesture, and it only
  fires once the fragment has already validated -- an invalid link never redirects, it shows the
  normal "looks broken" page instead. This is what the Facebook-audience case above wants. Since
  `try.freenet.org` is one shared peer, this suits occasional or small-audience sharing rather than
  a high-traffic post -- [freenet-core#5773](https://github.com/freenet/freenet-core/issues/5773)
  tracks scaling it up.
- **`?via=local`** -- same as no `via` at all (spelled out for clarity in a generated link): **Open
  on this computer** stays the primary button.
- **`?via=app`** -- makes **Open in Freenet** the primary button instead. Only worth setting if you
  know your audience is already on a release newer than 0.2.139; on an older release the button
  simply won't do anything, same as today.
- Anything else, or no `via` at all, falls back to the default above. Old links you've already
  shared are completely unaffected.

`via` is read from the query string, which -- unlike the fragment -- freenet.org's static host does
see (it'll show up in an access log as `?via=browser`, for instance). It only ever carries this one
word, never the target: the contract id, path and app fragment stay in the URL fragment exactly as
before, so freenet.org still never learns what any particular visitor opens. `via` doesn't create
any new destination either -- it only picks among the same three prefixes (`127.0.0.1:7509`,
`try.freenet.org`, `freenet:`) the page already uses.

## What it doesn't do

`/open` makes no network request of its own, and it doesn't try to detect whether Freenet is running
on the visitor's computer. Checking that would mean probing `localhost`, which leaks whether Freenet
is installed to anything embedding the page and, on some browsers, triggers a local-network access
prompt. Whoever built the app already knows how to turn a link back into a working session; `/open`
only carries the address there unread.
