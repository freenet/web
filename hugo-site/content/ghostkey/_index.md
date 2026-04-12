---
title: "Ghost Keys"
date: 2024-07-10
draft: false
layout: "single"
---

A Ghost Key is an **anonymous, verifiable identity** backed by a donation to Freenet.
You hold your Ghost Keys in a wallet — the Ghost Keys delegate — that runs inside your
Freenet node, and any Freenet app can ask to sign with one to prove you're a real human
without ever learning who you are.

### On this page

- [Why Ghost Keys exist](#why-ghost-keys-exist)
- [How it works: blind signing](#how-it-works-blind-signing)
- [Using Ghost Keys from a Freenet app](#using-ghost-keys-from-a-freenet-app)
- [What you can build with them](#what-you-can-build-with-them)
- [Storage, backup, and the CLI](#storage-backup-and-the-cli)
- [How much should I donate?](#how-much-should-i-donate)

You can also read the [introductory article](/about/news/introducing-ghost-keys/) or
[watch the interview](/about/news/ghost-keys-ian-interview/).

## Why Ghost Keys exist {#why-ghost-keys-exist}

There is no negative trust on the Internet. Identities are free to create, so a bad
reputation never sticks: spammers, bots, and Sybil attackers just spin up fresh accounts.
The usual fixes — captchas, phone numbers, "real name" policies — all trade your privacy
for a weak signal that you're human.

Ghost Keys take a different route. When you donate to Freenet, your browser mints a
cryptographic identity tied to that donation. The donation is the "skin in the game" —
you can't farm identities for free — but the donation and the identity are
**unlinkable**. The server that takes your money never sees the key it's authorising.

The result is an identity that is:

- **Anonymous** — no one, not even Freenet, can connect it back to you.
- **Scarce** — it costs real value to create, so Sybil attacks get expensive fast.
- **Portable** — it works across any Freenet app, and any app can verify it offline.

## How it works: blind signing {#how-it-works-blind-signing}

{{< ghostkeys-diagram-blindsign >}}

Your browser generates an Ed25519 keypair and
[blinds](https://en.wikipedia.org/wiki/Blind_signature) the public key before sending it
to the server. The server verifies the donation and signs the blinded key with its RSA
signing key. Your browser then unblinds the signature, producing a valid signature on
your real public key — one the server has never seen.

The signed public key, together with the donation amount and the delegate key that
signed it, forms your **Ghost Key certificate**. Anyone can verify it; no one can trace
it.

## Using Ghost Keys from a Freenet app {#using-ghost-keys-from-a-freenet-app}

Once imported, your Ghost Key doesn't just sit in a file. It lives inside the
**Ghost Keys delegate** — an agent that runs in a WASM sandbox inside your Freenet node.
Apps on Freenet talk to the delegate through a message API to request signatures; the
private key never leaves the sandbox.

{{< ghostkeys-diagram-delegate >}}

Under the hood, an app sends a `SignMessage` request with a scope (the app's identity,
attested by the runtime) and a payload. The delegate wraps the payload in a
`ScopedPayload`, signs it with your Ed25519 key, and returns a `SignResult` containing
the signature and your certificate. The first time a given app asks, you're prompted to
*allow once*, *always allow*, or *deny*.

Two properties matter here:

- **The key is unexportable.** Even an app running on your own machine can't extract it.
  The Freenet runtime enforces the sandbox; there is no API that hands out raw key
  material.
- **Signatures are scoped.** Because the runtime attests which app made the request,
  signatures are bound to that app. A malicious app can't harvest a signature and replay
  it somewhere else.

Verification works offline. Any recipient of a signed message can check the signature
and certificate with no call-home, no gatekeeper, and no dependency on Freenet being
online.

## What you can build with them {#what-you-can-build-with-them}

Ghost Keys are a primitive, not a product. A few of the things they unlock:

- **Spam-resistant chat and forums.** [River](https://freenet.org/river/) and other
  Freenet apps can require a Ghost Key to post, making flood attacks costly without
  tying posts to real-world identity.
- **Sybil-resistant voting and polling.** One donation, one voice — without a
  centralised voter roll.
- **Web-of-trust reputation.** Ghost Keys are stable, portable identities, so reputation
  can accumulate against them and travel between apps.
- **Paywall-free gated content.** Prove you contributed, without handing over an email
  or card.

## Storage, backup, and the CLI {#storage-backup-and-the-cli}

If you're running a Freenet node, click **Import to Freenet** on the success page after
donating — this installs your Ghost Key into the delegate on your node. We also
recommend downloading the certificate and signing key as a backup, so you can move your
identity to a new node later.

For developers, everything is open source:

- The [`freenet/ghostkeys`](https://github.com/freenet/ghostkeys) repository contains
  the delegate, the Dioxus UI, and the protocol types. This is what you integrate
  against if you're building a Freenet app.
- The [`ghostkey` CLI](https://crates.io/crates/ghostkey) lets you verify certificates
  and sign messages outside of Freenet — useful for scripts, CI, or non-Freenet tools
  that want to check Ghost Key signatures.

## How much should I donate? {#how-much-should-i-donate}

The minimum is **$1**. Donate as much as you can — the amount is recorded in your
certificate, so apps that want to grant additional privileges to larger or earlier
donors can do so. Think of it as a founding-member tier for the decentralised web.

<p style="text-align: center; margin: 2.5rem 0;">
<a href="/ghostkey/create/" class="funding-donate-button">Donate to Get Your Ghost Key</a>
</p>
