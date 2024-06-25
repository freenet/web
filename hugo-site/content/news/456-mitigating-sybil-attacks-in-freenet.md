+++
title = "Mitigating Sybil attacks in Freenet"
date = 2022-06-26
tags = ["front-page"]
+++

Every node in the Locutus network has a _location_, a floating-point value between 0.0 and 1.0 representing its position
in the small-world network. These are arranged in a ring so positions 0.0 and 1.0 are the same. Each contract also has a
location that is deterministically derived from the contract's code and parameters through a hash function.

The network's goal is to ensure that nodes close together are much more likely to be connected than distant nodes,
specifically, the probability of two nodes being connected
[should be](https://en.wikipedia.org/wiki/Small-world_routing) proportional to `1/distance`.

A [Sybil attack](https://en.wikipedia.org/wiki/Sybil_attack) is where an attacker creates a large number of identities
in a system and uses it to gain a disproportionately large influence which they then use for nefarious purposes.

In Locutus, such an attack might involve trying to control all or most peers close to a specific location. This could
then be used to drop or ignore get requests or updates for contract states close to that location.

# Solutions

1. [Increase the cost of creating a large number of nodes close to a specific chosen identities](#identity-creation-cost)
2. [Make it more difficult to target specific contracts](#location-hopping)
3. [Increase the cost of bad behavior by making other nodes in the network monitor for it and react accordingly](#peer-pressure)
4. [Use statistical anomaly detection to identify and thwart suspicious behavior](https://www.pivotaltracker.com/story/show/186472381)

## 1. Identity Creation Cost

### 1.1 Gateway assignment

When a node joins through a gateway it must negotiate its location with the gateway first. This could be done by both
node and gateway generating a random nonce, hashing it, and sending the hash to the other. After exchanging hashes they
exchange their actual nonces which are combined to create a new nonce, and a location is derived from that. This
prevents either gateway or the joiner from choosing the location.

#### 1.1.1 Attacks

- Gateway and joiner could collude to choose the location

### 1.2 IP-derived location

- The location could be derived deterministically from the node's IP address, this could then be verified by any node
  that communicates directly with it.

#### 1.2.1 Attacks

- Attackers could limit connections to peers they also control, which could then ignore a mismatch between their
  location and their IP address.

## 2. Location Hopping

### 2.1 Replication

A contract has multiple copies, each indicated by a contract parameter - the location of each copy will be pseudorandom.
A user could query a random subset of the copies to ensure that they receive any updates. If any copy has an old version
of the state then the user can update them by reinserting the latest version obtained from a different copy.

### 2.2 Date hopping

A contract contains a parameter for the current date, which will mean that the contract has a different location every
day. If today's contract is found to be missing it can be reinserted using an older copy.
