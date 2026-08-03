---
title: "Ghost Keys"
date: 2024-07-10
draft: false
layout: "single"
---

A Ghost Key is an **anonymous, verifiable identity** backed by a donation to Freenet.
You hold your Ghost Keys in the **Ghostkey Vault**, a Freenet delegate running inside
your Freenet node, and any Freenet app can ask the vault to sign with one to prove you
hold a scarce, donation-backed identity, without ever learning who you are.

<div class="gk-cta gk-cta-hero">
<a href="/ghostkey/create/" class="funding-donate-button">Get a Ghost Key</a>
<p class="gk-cta-note">$1 minimum. Freenet Project Inc is a 501(c)(3) nonprofit.</p>
</div>

## The short version

- **What you get.** An identity that any Freenet app can verify, and that no one can
  trace back to your payment.
- **What it costs.** $1 or more, once. There is no subscription and nothing renews.
- **Why a donation.** Identities that are free to create are free to farm. Paying for
  one makes spam and Sybil attacks expensive, and it funds Freenet's development.
- **How the anonymity works.** Your browser blinds the key before the donation server
  ever sees it, so the server signs a key it cannot read.
  [Details below](#how-it-works-blind-signing).
- **What it does not do.** It does not make everything you sign unlinkable. Two messages
  signed with the same Ghost Key can be tied to the same holder.
  [We are specific about this](#what-ghost-keys-dont-hide).

You can also read the [introductory article](/about/news/introducing-ghost-keys/) or
[watch the interview](/about/news/ghost-keys-ian-interview/).

## Why Ghost Keys exist

There is no negative trust on the Internet. Identities are free to create, so a bad
reputation never sticks: spammers, bots, and Sybil attackers just spin up fresh accounts.
The usual fixes (captchas, phone numbers, "real name" policies) all trade your privacy
for a weak signal that you're human.

Ghost Keys take a different route. When you donate to Freenet, your browser mints a
cryptographic identity tied to that donation. The donation is the "skin in the game"
(you can't farm identities for free), but the donation and the identity are
**unlinkable**: the server that takes your money never sees the key it's authorizing.

The result is an identity that is:

- **Anonymous at issue**: the issued key cannot be linked back to your donation;
  Freenet learns that someone donated, not who holds the resulting key. See
  [What Ghost Keys don't hide](#what-ghost-keys-dont-hide) below for the limits
  at use time.
- **Scarce**: it costs real value to create, so Sybil attacks get expensive fast.
- **Portable**: it works across any Freenet app, and any app can verify it offline.

### Why donations?

Freenet is built to remove central points of trust, so tying identity to a donation
processed by a central server is a deliberate design choice rather than an accident.
There are three reasons:

1. **It reuses off-the-shelf Sybil resistance.** Credit card networks already do
   meaningful identity work at the payment layer. Building a comparable decentralized
   mint is an open research problem we haven't solved.
2. **It funds the project.** Identity issuance becomes a revenue source for the rest
   of Freenet's work, instead of a cost center.
3. **It doesn't compromise anonymity.** Blind signing bounds the damage: the party
   that takes your money is cryptographically prevented from learning the key it's
   signing, so even a fully compromised donation server cannot correlate donors to
   Ghost Keys.

We're actively exploring decentralized alternatives; see
[Proof of Trust](/about/news/799-proof-of-trust-a-wealth-unbiased-consensus-mechanism-for-distributed-systems/).
Until one of them matures, the centralized mint is the pragmatic compromise.

## How it works: blind signing

{{< ghostkeys-diagram-blindsign >}}

Your browser generates an Ed25519 keypair and
[blinds](https://en.wikipedia.org/wiki/Blind_signature) the public key before sending
it to the donation server. The server verifies the donation and signs the blinded key
with its RSA signing key. Your browser then unblinds the signature, producing a valid
signature on your real public key, one the server has never seen.

The signed public key, together with the donation amount and the notary certificate
that signed it, forms your **Ghost Key certificate**. Anyone can verify the certificate
chains back to Freenet's master key; no one can link it to the donation that produced
it.

## What Ghost Keys don't hide

Blind signing protects the link between your donation and your Ghost Key: the donation
server never sees the key it is authorizing, so it cannot correlate donors to keys.
That guarantee holds at **issuance**.

It does not automatically hold at **use**. Once you use a Ghost Key to sign a message
that ends up in contract state (a chat post, a vote, a reputation claim), the key's
public half goes into that state, visible to anyone who reads the contract. Two messages
signed by the same Ghost Key are cryptographically linkable to the same holder. The link
is to a pseudonym, not to your real identity or your donation, but it is a persistent
pseudonym: activity across apps that share the same Ghost Key can be correlated by any
observer.

What this means in practice:

- **Cross-app correlation is possible.** If you use the same Ghost Key in two contracts,
  an observer of both contracts can tell it is the same holder.
- **Long-term linkability is possible.** Every message signed by a given key stays
  linked to that key for as long as the state exists.
- **Real-identity linkage is not automatic.** That still requires the pseudonym to leak
  through the content of what you sign, a side channel, or a deanonymization attack at
  the app layer.

The mitigation we are building toward is to **match key lifetime to the privacy unit
you actually want**. For apps where continuity is the feature, such as room membership
or long-lived reputation, a single stable Ghost Key is the right choice, and that is
what works today. For votes, ephemeral posts, and one-off signals, a fresh key per
action would be the right choice, and a design exists for it: a single donation issuing
a *bundle* of blinded Ghost Keys, so the economic model stays unchanged (you pay once
for a supply, not per stamp) while apps get per-action unlinkability. The Ghostkey Vault
already supports holding multiple keys.

That design is **parked rather than in progress**, and it is worth being straight about
why. Ghost Keys are principally a replacement for CAPTCHAs on one-off actions, and they
already do that job. Per-action unlinkability is an improvement on it, not a missing
prerequisite — so it waits for an app that actually needs it. The reasoning and the
measurements are recorded in
[freenet/ghostkeys#2](https://github.com/freenet/ghostkeys/issues/2).

A stronger mitigation would be to prove that a valid Ghost Key signed a message
*without revealing the key itself*, using a zero-knowledge proof over the certificate.
This has been prototyped far enough to measure: a proof is around 400 bytes and takes
roughly 84 ms to verify inside a Freenet contract. The obstacle is not speed but
structure — contract state is re-validated on every load, so verification has to happen
once at admission rather than per read, and a contract cannot do that on its own. It is
a real option rather than a plan, and it is parked alongside bundling for the same
reason.

<div class="gk-cta">
<a href="/ghostkey/create/" class="funding-donate-button">Get a Ghost Key</a>
<p class="gk-cta-note">$1 minimum. You now know both what it protects and what it doesn't.</p>
</div>

## Using Ghost Keys from a Freenet app

Once imported, your Ghost Key doesn't just sit in a file. It lives inside the
**Ghostkey Vault**, a Freenet delegate (a sandboxed WASM agent running inside
your Freenet node). Apps on Freenet talk to the vault through a message API to request
signatures; the private key never leaves the sandbox.

{{< ghostkeys-diagram-delegate >}}

Under the hood, an app sends a `SignWithDefault` request carrying a payload. The vault
wraps it in a `ScopedPayload` alongside the app's identity (attested by the runtime),
signs it with your Ed25519 key, and returns a `SignResult` containing the signature and
your certificate. The first time a given app asks, you're prompted to pick a key, or to
deny. An app that already knows which key it wants can name one with `SignMessage`
instead.

Apps can also ask `HasIdentity` — a question rather than a request, answered without
prompting you — so they can decide whether to offer you a Ghost Key path at all before
putting any dialog in front of you.

Two properties matter here:

- **The private key is inaccessible to apps.** Freenet apps running on your node cannot
  extract key material through the vault's API; the runtime enforces the sandbox and
  there is no call that hands out the raw key. (You can still back up your own key file
  separately; see below.)
- **Signatures are scoped.** The runtime attests which app made each signing request,
  and the vault embeds that scope in the signed payload. As long as verifiers check
  the scope, a malicious app can't harvest a signature and replay it against a
  different app.

Verification works offline. Any recipient of a signed message can check the signature
and certificate with no call-home, no gatekeeper, and no dependency on Freenet being
online.

## What you can build with them

Ghost Keys are a primitive, not a product. A few of the things they unlock:

- **Spam-resistant chat and forums.** [River](https://freenet.org/river/) and other
  Freenet apps can require a Ghost Key to post, making flood attacks costly without
  tying posts to real-world identity.
- **Sybil-resistant voting and polling.** One Ghost Key, one voice; additional votes
  cost additional donations, cheap for individuals and expensive at scale.
- **Web-of-trust reputation.** Ghost Keys are stable, portable identities, so
  reputation can accumulate against them and travel between apps.
- **Paywall-free gated content.** Prove you contributed, without handing over an email
  or card.

## Storage, backup, and the CLI

If you're running a Freenet node, click **Import to Freenet** on the success page after
donating; this installs your Ghost Key into the Ghostkey Vault on your node.

**Back it up.** The vault marks a newly imported identity as un-backed-up and shows a
reminder next to it with a one-click download, which clears only once you confirm you
have the file. Treat that as the point of the exercise rather than a nag: on most setups
the vault holds the only copy, some nodes reclaim idle storage, and a key you cannot
produce is a donation you cannot prove. A backup also lets you move your identity to a
new node later.

### Opening your vault

If you have a Freenet node running on this computer, your Ghost Keys are here:

<div class="gk-cta">
<a href="http://localhost:7509/v1/contract/web/DLog47hEsrtuGT4N5XCeMBG45m4n1aWM89tBZXue2E1N/" class="funding-donate-button">Open your Ghost Key vault</a>
<p class="gk-cta-note">Requires a Freenet node running on this computer. The link will not resolve otherwise.</p>
</div>

That address is your own machine, not a website — the vault runs inside your node, and the page is
served locally. Bookmark it if you use Ghost Keys regularly.

For developers, everything is open source:

- The [`freenet/ghostkeys`](https://github.com/freenet/ghostkeys) repository contains
  the Ghostkey Vault, its Dioxus UI, and the protocol types. This is what you integrate
  against if you're building a Freenet app.
- The [`ghostkey` CLI](https://crates.io/crates/ghostkey) lets you verify certificates
  and sign messages outside of Freenet, useful for scripts, CI, or non-Freenet tools
  that want to check Ghost Key signatures.

## How much should I donate?

The minimum is **$1** and the maximum is **$10,000**. Donate as much as you can; the
amount is recorded in your certificate, so apps that want to grant additional privileges
to larger donors can do so.

Card issuers commonly decline single charges in the thousands, so the two largest tiers
may not go through on an ordinary consumer card. A decline costs you nothing — no charge
is made and no key is issued — so it is safe to try one and fall back to a smaller
amount.

### Why the amounts are fixed

You choose from $1, $5, $20, $50, $100, $500, $2,500, or $10,000 rather than typing your
own figure, and that is a privacy decision rather than a limitation we haven't got around
to lifting.

The donation amount is written into your certificate, so anyone who verifies it can read
it. Fixed tiers mean your certificate is indistinguishable from every other certificate
issued at the same amount: the amount tells an observer which of eight groups you are in
and nothing more. A free-form amount like $37.42 would be close to unique, and would
give away much of what blind signing is there to protect.

The same logic applies to the tiers themselves: the larger ones will have fewer holders,
so a certificate at the top of the range says more about you than one at the bottom.
Nobody outside the Freenet Project can turn that into a name — doing so needs the
payment records, which never leave us and are never linked to a certificate — but if you
want the largest possible crowd to hide in, the lower tiers are where it is.

<div class="gk-cta">
<a href="/ghostkey/create/" class="funding-donate-button">Get a Ghost Key</a>
<p class="gk-cta-note">$1 minimum. Freenet Project Inc is a 501(c)(3) nonprofit.</p>
</div>
