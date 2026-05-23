---
title: "Apps & Ecosystem"
date: 2026-05-23
draft: false
layout: "single"
aliases:
  - /build/example-apps/
  - /dev/apps/
---

The apps below run on Freenet rather than on a company's servers. They are
unstoppable, interoperable by default, and built on the same open protocol.
Everything in Freenet's ecosystem (apps, tools, components) can interoperate
without coordination, so the line between "an app" and "a building block" is
deliberately thin.

## Available now

### River {#river}

Decentralized group chat. Encrypted rooms that work even if every Freenet
contributor disappears tomorrow. The fastest way to see Freenet in action: the
[Quickstart](/quickstart/) joins you to the live developers' room.

{{< app-screenshot light="/images/apps/river.png" alt="River decentralized group chat interface" >}}

→ [github.com/freenet/river](https://github.com/freenet/river)

### Delta {#delta}

Decentralized website builder and publishing platform. Author and publish sites
that live across the peer-to-peer network with no hosting bills and no
take-down vector.

→ [github.com/freenet/delta](https://github.com/freenet/delta)

### Mail {#mail}

Decentralized email built on Freenet. Messages routed peer-to-peer with no
provider in the middle.

→ [github.com/freenet/mail](https://github.com/freenet/mail)

### freenet-git {#freenet-git}

Decentralized Git hosting and collaboration over Freenet. Push and clone
without depending on a central forge.

→ [github.com/freenet/freenet-git](https://github.com/freenet/freenet-git)

## In development

### Harvest {#harvest}

Decentralized marketplace for peer-to-peer commerce. Anonymous,
donation-backed identities paired with a blind-signature feedback mechanism
provide accountability without a central operator. Early design.

→ [github.com/freenet/harvest](https://github.com/freenet/harvest)

### Atlas {#atlas}

Decentralized discovery layer: a framework for publishing signed metadata
about Freenet content and building competing, pluralistic search,
recommendation, and curation systems on top. Early RFC.

{{< app-screenshot light="/images/apps/atlas.png" alt="Atlas discovery UI mockup" >}}

The screenshot above is an early design mockup, not a working UI. Atlas is
still at the RFC stage.

→ [github.com/freenet/atlas](https://github.com/freenet/atlas)

### Raven {#raven}

Decentralized live social feed (posts, profiles, follows, likes), designed as
a high-churn live surface that complements [Atlas](#atlas) for durable
discovery and archival.

{{< app-screenshot light="/images/apps/raven.png" alt="Raven microblogging interface" >}}

→ [github.com/freenet/raven](https://github.com/freenet/raven)

## Tools & components

### Ghost Keys {#ghost-keys}

Anonymous, Sybil-resistant identity certificates. Donate a small amount to
mint a key; apps can verify the certificate without learning who you are. Used
across Freenet apps as a spam- and abuse-resistance primitive.

→ [Get a Ghost Key](/ghostkey/) · [github.com/freenet/ghostkeys](https://github.com/freenet/ghostkeys)

### freenet-scaffold {#freenet-scaffold}

A Rust utility crate that simplifies building Freenet contracts with efficient,
mergeable state synchronization.

→ [github.com/freenet/freenet-scaffold](https://github.com/freenet/freenet-scaffold)

---

Building something new on Freenet? See the [tutorial](/build/manual/tutorial/)
and [manual](/build/manual/) to get started, or open a PR to add your project
to this page.
