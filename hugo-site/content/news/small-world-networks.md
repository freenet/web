---
title: "Understanding Small World Networks"
date: 2024-11-25
draft: false
tags: ["front-page", "university"]
head:
  - - link
    - rel: stylesheet
      href: https://cdnjs.cloudflare.com/ajax/libs/font-awesome/5.15.4/css/all.min.css
---

In the 1960s, psychologist Stanley Milgram conducted an influential experiment that revealed just
how interconnected our world really is. Milgram repeated this letter-sending experiment nearly 200
times, where participants were asked to forward a letter to a target individual, but only by passing
it to someone they personally knew. On average, these letters reached their target in just six
steps, giving rise to the concept of 'six degrees of separation.' Milgram's findings demonstrated
that despite the vastness of the world, most individuals are only a few links away from each other,
highlighting the surprisingly small number of intermediaries connecting us all.

So, how does this concept apply to technology and Freenet? Freenet uses principles similar to those
observed in Milgram's experiment to efficiently locate information in a decentralized way. Each node
in Freenet is connected to a limited number of other nodes, and much like in a small world social
network, this network structure allows data to be found with minimal hops, even when the network
scales to millions of nodes. This efficient discovery mechanism is what makes Freenet both
decentralized and highly scalable, embodying the core principles of a small world network.

## Small World Networks: Combining Regularity and Randomness

Small world networks combine the strengths of both regular and random networks. In a regular
network, nodes are primarily connected to their immediate neighbors, creating an orderly but often
slow communication structure. On the other hand, random networks have many short paths between
nodes, but they tend to be disorganized and unreliable. Small world networks strike a balance
between these extremes by featuring mostly local connections along with a few strategically placed
long-range links, which significantly reduce the average path length.

To better understand this, let's look at an animation that visualizes a key property of these
networks: **Greedy Routing**.

## Greedy Routing: Finding a Path With Local Knowledge

A defining characteristic of small world networks is their ability to support efficient "greedy"
routing. In this context, "greedy" routing refers to a process where each node forwards a message to
the neighbor that appears to be closest to the final destination, based only on locally available
information:

{{< small-world-routing >}}

In this animation, you can observe messages navigating the network using only local decisions at
each step. While this approach does not always yield the mathematically shortest path, it is
remarkably effective at navigating small world structures. Freenet leverages a similar principle:
nodes use their local knowledge to make efficient routing decisions, much like the participants in
Milgram's experiment passing letters through their personal networks.

## Comparing Routing Performance: Small World vs. Random Networks

The benefits of small world networks are most apparent when we compare their routing performance
with that of purely random networks. The animation below illustrates how both network types route
messages between random pairs of nodes:

{{< small-world-comparison >}}

The small world network is shown on the left, while the random network is on the right. Notice how
the small world network consistently achieves higher success rates in delivering messages. This is
due to its strategic combination of short- and long-range connections, which naturally create
efficient routing pathways. In contrast, the random network's unstructured connections often lead to
inefficient, unreliable paths.

In our comparison, a routing attempt is deemed successful if the message reaches its destination
within 30 hops. The structured randomness of the small world network ensures more reliable message
delivery, whereas the random network frequently requires more hops or fails to find a path
altogether.

## Scaling Small World Networks: Efficient Growth

One of the most remarkable aspects of small world networks is how efficiently they scale as new
nodes are added. The next animation shows the average path length as the network grows:

{{< small-world-scale >}}

In small world networks, the average path length grows logarithmically with the number of nodes,
which is far more efficient than the linear growth typical of regular networks. This means that even
as the network expands to include millions of nodes, the average number of steps required to reach
any other node remains relatively low—a critical feature for scalability. This property is precisely
what allows Freenet to maintain efficient data retrieval, regardless of how large the network
becomes.

The blend of local and long-range connections in small world networks creates a structure that is
both resilient and highly efficient. This pattern has been discovered repeatedly in nature, from
social networks to neural pathways. Freenet leverages this natural efficiency to create a truly
decentralized and scalable method for sharing information.
