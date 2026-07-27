---
title: "Research Papers"
date: 2026-07-27
draft: false
description: "Academic papers describing the original Freenet design, its small-world routing algorithms, and measurements of the deployed network."
aliases:
  - /papers/
---

These papers describe the **original** Freenet, the design Ian Clarke started in
1999 at the University of Edinburgh. In March 2023 that original version was spun
off as an independent project called [Hyphanet](https://www.hyphanet.org/), and
the ground-up redesign that had been developed internally as "Locutus" took the
Freenet name. See [Freenet's History](/about/history/) and the
[FAQ](/about/faq/#why-was-freenet-rearchitected-and-rebranded) for how that
happened and why.

None of the papers below describe the software you install from this site today.
For the current architecture, read the [Freenet whitepaper](/whitepaper/).

They are still worth reading. The problems they work on, routing without global
knowledge, building a searchable network out of connections you do not get to
choose, and measuring what a deployed anonymous network actually does, did not
go away, and some of the results carry over to systems with no connection to
Freenet at all. [Distributed Routing in Small-World Networks](#small-world-routing)
is the clearest example: the same algorithm applies to any network whose topology
you inherit rather than choose, including a wireless mesh.

We host copies here because these papers are part of the project's record and we
would rather they stay available than depend on any one site. The originals are
linked from [Hyphanet's about page](https://www.hyphanet.org/pages/about.html).

## The original Freenet

**[A Distributed Decentralised Information Storage and Retrieval System](/pdf/DDISRS.pdf)** (PDF, 45 pages)
Ian Clarke, Division of Informatics, University of Edinburgh, 1999.

The undergraduate project report that started Freenet.

**[Freenet: A Distributed Anonymous Information Storage and Retrieval System](/pdf/ADAISARS.pdf)** (PDF, 21 pages)
Ian Clarke, Oskar Sandberg, Brandon Wiley, Theodore W. Hong, 2001.

The paper most people mean by "the Freenet paper." It has been
[cited over 3,700 times](https://scholar.google.com/scholar?cluster=17926651926152536224&hl=en&as_sdt=0,44),
which puts it among the most cited computer science papers of its year.

**[Protecting Free Expression Online with Freenet](/pdf/papers/freenet-ieee.pdf)** (PDF, 10 pages)
Ian Clarke, Scott G. Miller, Theodore W. Hong, Oskar Sandberg, Brandon Wiley.
IEEE Internet Computing, January/February 2002.

A shorter, more accessible description of the architecture as it stood in 2002.
A good starting point if you want the ideas without the formalism.

## Small-world routing

**[Distributed Routing in Small-World Networks](/pdf/papers/swroute.pdf)** (PDF, 12 pages)
Oskar Sandberg, 2005.

Greedy routing in a small-world network normally assumes every node knows where
it sits relative to the destination. This paper drops that assumption. It gives
a Markov Chain Monte Carlo algorithm that takes only the graph as input, assigns
positions to nodes, and recovers efficient greedy routing, with no global
coordination and no knowledge of node positions given in advance. It is the
theoretical basis for the "darknet" routing used in Freenet 0.7.

The reason this one is still interesting has little to do with anonymity. Ian
Clarke, commenting on the paper recently:

> It shows how Freenet's small-world network can be adapted to situations where
> the network topology is predetermined. In this paper, that's because the
> connections correspond to human relationships, but the same approach could
> equally be applied to a mesh network, where connectivity is determined by
> geography.

If you cannot choose your neighbors, whether because they are your friends or
because they are the only radios in range, this is the algorithm that makes the
resulting network routable.

**[Searching in a Small World](/pdf/papers/lic.pdf)** (PDF, 78 pages)
Oskar Sandberg, licentiate thesis, Chalmers University of Technology and
Göteborg University, 2005.

The longer treatment. It describes a decentralized mechanism for constructing
small-world networks, inspired by the original Freenet's design; Part II covers
the basis for the darknet architecture.

**[Switching for a Small World](/pdf/papers/vilhelm_thesis.pdf)** (PDF, 36 pages)
Vilhelm Verendel, master's thesis in Complex Adaptive Systems, Chalmers
University of Technology, 2007.

Explores ways to optimize the swapping algorithm from the papers above.

## The darknet

**[Private Communication Through a Network of Trusted Connections: The Dark Freenet](/pdf/papers/freenet-0.7.5-paper.pdf)** (PDF, 19 pages)
Ian Clarke, Oskar Sandberg, Matthew Toseland, Vilhelm Verendel, 2010.

Describes the Freenet 0.7.5 architecture: a network where each node connects
only to peers its operator already trusts, so running a node reveals your
identity only to people you have chosen. Routing over that fixed mesh uses the
algorithm from the small-world papers above. Includes simulations under
realistic conditions.

## Measurement

**[Measuring Freenet in the Wild: Censorship-resilience under Observation](/pdf/papers/roos-pets2014.pdf)** (PDF, 20 pages)
Stefanie Roos, Benjamin Schiller, Stefan Hacker, Thorsten Strufe.
Privacy Enhancing Technologies Symposium (PETS), 2014.

An independent measurement study of the deployed network, with a code analysis
identifying bottlenecks. Two findings stand out: the topology control mechanisms
in use were suboptimal for routing, and the network had tens of thousands of
users whose online times were unusually long compared to other peer-to-peer
systems. Worth reading alongside the design papers, since it shows where the
design and the deployment diverged.
