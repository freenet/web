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

<div style="float: right; margin-left: 20px; margin-bottom: 10px; max-width: 300px; width: 100%;">
    <img src="/img/handing-letter-sw.webp" alt="Handing a Letter" style="width: 100%; border: 1px solid #ccc; border-radius: 5px; box-shadow: 2px 2px 10px rgba(0,0,0,0.1);">
</div>

In the 1960s psychologist Stanley Milgram conducted an influential experiment that revealed
something amazing about human relationships. Milgram chose people at random in cities like Kansas
and gave each a letter with the address of someone they didn't know in Boson, Massachusetts. They
were instructed to get the letter to that person but only by sending it to someone they know
personally, who would send it to someone they know personally - and so on. Milgram repeated this
letter-sending experiment nearly 200 times. On average, these letters reached their target in just
six steps, this is where we get the term 'six degrees of separation.' Milgram's findings
demonstrated that despite the vastness of the world, most individuals are only a few links away from
each other, highlighting the surprisingly small number of intermediaries connecting us all.

Freenet uses principles similar to those observed in Milgram's experiment to efficiently locate
information in a decentralized way. Each node in Freenet is connected to a limited number of other
nodes, and much like in a small world social network, this network structure allows data to be found
with minimal hops, even when the network scales to millions of nodes. This efficient discovery
mechanism is what makes Freenet both decentralized and highly scalable.

### Small World Networks: Combining Regularity and Randomness

Small world networks combine the strengths of both regular and random networks. In a regular
network, nodes are primarily connected to their immediate neighbors, creating an orderly but often
slow communication structure. On the other hand, random networks have many short paths between
nodes, but they tend to be disorganized and unreliable. Small world networks strike a balance
between these extremes by featuring mostly local connections along with some long-range links, which
significantly reduce the average path length.

To better understand this, let's look at an animation that visualizes a key property of these
networks: **Greedy Routing**.

### Greedy Routing: Finding a Path With Local Knowledge

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

### Comparing Routing Performance: Small World vs. Random Networks

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
nodes are added. In small world networks, the average path length grows logarithmically with the
number of nodes, meaning that as the network doubles in size, the average number of steps to reach
any node only increases by a small amount. This contrasts with linear growth, where doubling the
size of the network would double the number of steps required. This logarithmic growth is crucial
because it ensures that even as the network expands to include millions of nodes, the average path
length remains manageable.

The next animation shows the average path length as the network grows.

{{< small-world-scale >}}

The blend of local and long-range connections in small world networks creates a structure that is
both resilient and highly efficient. This pattern has been discovered repeatedly in nature, from
social networks to neural pathways. Freenet leverages this natural efficiency to create a truly
decentralized and scalable method for sharing information.

## Learn more

This article just scratches the surface, for much more detail on how Freenet uses small world
networks please watch Ian's video talk [How
Freenet Works]({{< ref "/goals/news/how-freenet-works-video-talk.md" >}}).
