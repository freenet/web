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

You can also read the [introductory article](/about/news/introducing-ghost-keys/) or
[watch the interview](/about/news/ghost-keys-ian-interview/).

> **⚠️ Back up your Ghost Key immediately after creating it.** The Ghostkey Vault
> delegate is still early software and storage is not yet reliable: keys can and do
> disappear from the vault. When you receive your certificate and signing key, save
> both to a password manager or other secure backup **before** importing to Freenet.
> Without a backup, a lost key cannot be recovered and the donation behind it is gone.

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
action is the right choice, and that is on our roadmap: a single donation will issue
a *bundle* of blinded Ghost Keys so the economic model stays unchanged (you pay once
for a supply, not per stamp) while apps get per-action unlinkability. The Ghostkey
Vault already supports holding multiple keys; the missing piece is bundled issuance.
Tracked in [freenet/ghostkeys#2](https://github.com/freenet/ghostkeys/issues/2).

A stronger mitigation is on the roadmap as a longer-horizon design direction: proving
that a valid Ghost Key signed a message *without revealing the key itself*, using a
zero-knowledge proof over the certificate. It would require redesigning the signature
format and the verifier, and is a substantial piece of work, but it is the direction
we expect Ghost Keys to move in.

## Using Ghost Keys from a Freenet app

Once imported, your Ghost Key doesn't just sit in a file. It lives inside the
**Ghostkey Vault**, a Freenet delegate (a sandboxed WASM agent running inside
your Freenet node). Apps on Freenet talk to the vault through a message API to request
signatures; the private key never leaves the sandbox.

{{< ghostkeys-diagram-delegate >}}

Under the hood, an app sends a `SignMessage` request with a scope (the app's identity,
attested by the runtime) and a payload. The vault wraps the payload in a
`ScopedPayload`, signs it with your Ed25519 key, and returns a `SignResult` containing
the signature and your certificate. The first time a given app asks, you're prompted
to *allow once*, *always allow*, or *deny*.

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

**Back up first, import second.** On the success page after donating, save both the
certificate and the signing key to a password manager (or another secure location)
**before** you click **Import to Freenet**. The Ghostkey Vault delegate is still
early software: keys have been observed disappearing from the vault after import, and
without a saved backup the key (and the donation behind it) cannot be recovered. Treat
the vault as a convenience, not as the authoritative store of your key, until this is
fixed. Progress is tracked in
[freenet/ghostkeys#3](https://github.com/freenet/ghostkeys/issues/3).

For developers, everything is open source:

- The [`freenet/ghostkeys`](https://github.com/freenet/ghostkeys) repository contains
  the Ghostkey Vault, its Dioxus UI, and the protocol types. This is what you integrate
  against if you're building a Freenet app.
- The [`ghostkey` CLI](https://crates.io/crates/ghostkey) lets you verify certificates
  and sign messages outside of Freenet, useful for scripts, CI, or non-Freenet tools
  that want to check Ghost Key signatures.

## How much should I donate?

The minimum is **$1**. Donate as much as you can; the amount is recorded in your
certificate, so apps that want to grant additional privileges to larger donors can do
so.

<p style="text-align: center; margin: 2.5rem 0;">
<a href="/ghostkey/create/" class="funding-donate-button">Donate to Get Your Ghost Key</a>
</p>
