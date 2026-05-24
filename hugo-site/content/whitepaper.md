---
title: "Freenet Whitepaper"
date: 2026-05-24
draft: false
type: "page"
description: "A peer-to-peer platform for real-time decentralized applications: idempotent commutative monoids and adaptive small-world routing."
---

A whitepaper describing the architecture of Freenet: contracts as
application-defined join-semilattices, summary/delta synchronization,
small-world adaptive routing, and the delegate model for private state.

<div style="margin: 1.5em 0; display: flex; flex-wrap: wrap; gap: 1em;">
  <a href="/pdf/freenet-whitepaper.pdf"
     style="display: inline-block; padding: 0.6em 1.2em; background: #2a5b8a; color: white; text-decoration: none; border-radius: 4px; font-weight: 600;">
    Read the whitepaper (PDF)
  </a>
  <a href="https://github.com/freenet/paper-1"
     style="display: inline-block; padding: 0.6em 1.2em; background: #444; color: white; text-decoration: none; border-radius: 4px; font-weight: 600;">
    View source on GitHub
  </a>
</div>

## What it covers

The whitepaper is design-oriented. It describes the architecture as currently
implemented in the `freenet-core` reference runtime, summarizes a 24-hour
live-network measurement of routing path lengths, and is explicit about which
mechanisms are deployed, which remain experimental, and which are open problems.

Section headings:

1. The problem
2. Design thesis
3. Core primitives (peers, contracts, delegates, user interfaces)
4. How updates move (the merge algebra, summary/delta sync, subscription trees)
5. How data is found (small-world routing, adaptive routing, live-network measurement)
6. Security and trust boundaries
7. Implementation status and open problems
8. Related work
9. Conclusion

## Source and rebuilds

The whitepaper source lives at
[github.com/freenet/paper-1](https://github.com/freenet/paper-1) as a small
LaTeX project. A GitHub Actions workflow rebuilds the PDF on every push to
`main`; the latest build is also attached to the
[`latest` release](https://github.com/freenet/paper-1/releases/tag/latest).
Pull requests are welcome.

## Citing

```
Clarke, I. (2026). Freenet: A Peer-to-Peer Platform for Real-Time
Decentralized Applications. https://freenet.org/whitepaper/
```
