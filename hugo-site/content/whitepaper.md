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

<div style="margin: 1.5em 0;">
  <a href="/pdf/freenet-whitepaper.pdf"
     style="display: inline-block; padding: 0.6em 1.2em; background: #2a5b8a; color: white; text-decoration: none; border-radius: 4px; font-weight: 600;">
    Read the whitepaper (PDF)
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

Source: [github.com/freenet/paper-1](https://github.com/freenet/paper-1).

## Citing

```
Clarke, I. (2026). Freenet: A Peer-to-Peer Platform for Real-Time
Decentralized Applications. https://freenet.org/whitepaper/
```

There is also an [AI-generated audio overview](https://www.youtube.com/watch?v=DZNedczsHuY)
of the paper produced by Google NotebookLM. The paper itself is the canonical
source; the overview is less precise.
