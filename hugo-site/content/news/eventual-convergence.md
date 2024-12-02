---
title: "Understanding Summary-Delta Synchronization"
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

### How Freenet's Eventual Convergence Sidesteps This Challenge

Instead of relying on heavyweight consensus mechanisms, Freenet adopts an **eventual consistency**
model, but with a unique twist called **Eventual Convergence**. Here’s why this approach is
especially powerful and flexible:

#### 1. Flexible Merge Mechanism Defined by Contracts

In Freenet, every value stored under a given key is required to be **mergeable**—meaning that
different versions can be combined into a consistent state. But instead of enforcing a rigid,
one-size-fits-all merge strategy, Freenet uses **WebAssembly (Wasm) contracts** to define how data
should be synchronized. The author of each Wasm contract specifies the rules for merging data,
allowing them to tailor the synchronization process to the specific needs of the application. This
flexibility is crucial because not all data requires the same kind of consistency. For some use
cases, merging could mean taking the union of two sets, while for others, it might involve choosing
the latest timestamp.

#### 2. Efficient Synchronization via Summary and Delta

Freenet’s approach involves a **two-step process** that ensures eventual convergence efficiently:

- Each node generates a **summary** of its current state, which is a compact representation of what
  it knows.
- Nodes exchange these summaries, allowing them to create a **delta**—the set of changes needed to
  bring their state up to date with the other node's state.

These summaries and deltas can be extremely efficient because they’re represented as arbitrary byte
arrays, and their structure is defined by the Wasm contract. This efficiency means that nodes can
converge without having to exchange large amounts of redundant information.

#### 3. Summary-Delta Synchronization in a Small-World Network

In Freenet, the key-values are stored using a **small-world network** topology, which has
interesting properties for distributed consistency. For a given key, nodes subscribe to the value,
forming a connected "tree" structure, with the root being the node closest to the key. Updates
propagate through this tree using the **Summary-Delta Convergence** mechanism—similar to a virus
spreading through a network. This ensures that changes are efficiently propagated to all subscribing
nodes, achieving eventual consistency over time.

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

Thus, **Freenet's Summary-Delta Synchronization** not only simplifies the problem of achieving eventual consistency in
distributed systems but also provides the flexibility and efficiency needed for diverse,
decentralized applications.

#### Peer Synchronization Example

Below is a simple visualization of how two peers synchronize their data using summaries and deltas:

{{< eventual-convergence/sync >}}

1. Each peer starts with different data (represented by icons)
2. They generate summaries of their data (lists of icon names)
3. By comparing summaries, each peer determines what data to send (the delta)
4. After exchanging deltas, both peers have the same complete set of data

#### Network-wide Propagation

The visualization below shows how updates propagate through the entire network:

{{< eventual-convergence/propagation >}}

Click different nodes to see how information spreads through the network. Notice how:

- Updates propagate gradually from node to node
- Multiple updates can spread simultaneously
- The network eventually reaches a consistent state
- Each node maintains a history of updates it has received

More detailed explanations and additional visualizations coming soon...
