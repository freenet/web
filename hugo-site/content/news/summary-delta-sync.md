---
title: "Understanding Freenet's Delta-Sync"
date: 2024-11-30
draft: false
tags: ["front-page", "university"]
head:
  - - link
    - rel: stylesheet
      href: https://cdnjs.cloudflare.com/ajax/libs/font-awesome/5.15.4/css/all.min.css
---

### The Challenge of Consistency in Distributed Systems

Achieving consistency across distributed systems is a notoriously difficult problem. The key reason
is that, in a distributed environment, multiple nodes can independently make changes to the same
piece of data. When different nodes hold different versions of this data, deciding how to reconcile
these differences without losing valuable updates or introducing conflicts becomes a complex
challenge.

Traditional approaches often require coordination mechanisms, such as **consensus algorithms** (like
Paxos or Raft), to ensure consistency. However, these methods can be resource-intensive, require
high communication overhead, and often struggle with scalability, especially when dealing with
frequent updates across many nodes. The famous **CAP theorem** even states that distributed systems
can only guarantee two of three properties—**Consistency, Availability, and Partition Tolerance**—at
any given time, making it hard to achieve strong consistency while keeping a system always available
and partition-tolerant.

### How Freenet Sidesteps This Challenge

Instead of relying on heavyweight consensus mechanisms, Freenet adopts a (to our knowledge) novel
**eventual consistency** approach. Here’s why this approach is especially powerful and flexible:

#### 1. Flexible Merge Mechanism Defined by Contracts

In Freenet, every value stored under a given key must be **mergeable**, meaning that different
versions can be combined into a consistent state. To ensure consistency, merging must be
order-independent, always producing the same result regardless of the sequence in which states are
combined (a property known as a commutative monoid). Rather than imposing a rigid, universal merge
strategy, Freenet leverages **WebAssembly (Wasm) contracts** to define custom synchronization rules.
Each Wasm contract is authored to specify how data should be merged, allowing synchronization to be
tailored to the unique requirements of the application. This flexibility is essential because
different types of data demand different approaches to consistency. For example, merging might
involve taking the union of two sets for one use case, while another might prioritize the version
with the latest timestamp.

#### 2. Efficient Synchronization via Summary and Delta

Freenet’s approach involves a **two-step process** designed to minimize the amount of data
transferred.

- Each node generates a **summary** of its current state, which is a compact representation of what
  it knows.
- Nodes exchange these summaries, allowing them to create a **delta**—the set of changes needed to
  bring their state up to date with the other node's state.

These summaries and deltas can be extremely efficient because they’re represented as arbitrary byte
arrays, and their structure is defined by the Wasm contract.

#### 3. Summary-Delta Synchronization in a Small-World Network

In Freenet, the key-values are stored using a [small
world]({{< relref "small-world-networks.md" >}}) topology, which has interesting properties for distributed
consistency. For a given key, nodes subscribe to the value, forming a connected "tree" structure, with
the root being the node closest to the key. Updates propagate through this tree using a mechanism—similar
to a virus spreading through a network. This ensures that changes are efficiently propagated to all subscribing
nodes, quickly achieving eventual consistency.

#### Illustrating Summary-Delta Synchronization: The Color Mixing Analogy

To help understand how multiple updates can occur simultaneously yet lead to the same end result, we
can use a **color mixing** analogy. Imagine updates represented by different colors spreading
through the tree of nodes. As updates propagate and meet at nodes, their colors mix. Even though the
updates might reach different nodes in different orders, the merging process ensures that, in the
end, each node arrives at the same color—demonstrating a consistent and commutative outcome.

By using **Summary-Delta Synchronization**, Freenet sidesteps the traditional difficulties of strong
consistency:

- **No Need for Heavy Coordination**: Instead of relying on consensus, Freenet nodes independently
  merge states in a way that eventually leads to the same outcome across the network.
- **Flexibility via Contracts**: The Wasm contracts offer an extremely flexible way for data authors
  to specify how synchronization should occur, allowing each application to define its own
  consistency rules.
- **Scalability**: This mechanism scales well because it avoids constant global coordination,
  relying instead on local operations and efficient propagation through a structured small-world
  network.

**Freenet's Summary-Delta Synchronization** not only simplifies the problem of achieving eventual
consistency in distributed systems but also provides the flexibility and efficiency needed for
diverse, decentralized applications.

#### Peer Synchronization Example

Below is a simple visualization of how two peers synchronize their data using summaries and deltas:

{{< summary-delta-sync/sync >}}

1. Each peer starts with different data (represented by icons)
2. They generate summaries of their data (lists of icon names)
3. By comparing summaries, each peer determines what data to send (the delta)
4. After exchanging deltas, both peers have the same complete set of data

#### Subscription Tree Propagation

The visualization below shows how updates propagate through a contract subscription tree using color
to represent state updates, just click individual nodes to trigger updates. Notice how:

- Updates propagate from node to node like a virus
- Multiple updates can spread simultaneously
- The network quickly reaches a consistent state

{{< summary-delta-sync/propagation >}}
