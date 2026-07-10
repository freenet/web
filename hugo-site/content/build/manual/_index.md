---
title: "Freenet Manual"
date: 2025-04-13
draft: false
layout: "single"
aliases:
  - /resources/manual/
  - /manual/
---

This guide provides **comprehensive documentation** on Freenet's components, architecture, and
usage. Use the table of contents below to navigate through the manual.

---

## Table of Contents

1. [Introduction](introduction)
2. [Components](#components)
3. [Architecture](#architecture)
4. [Developer Guide](#developer-guide)
5. [Client SDKs](#client-sdks)
6. [Examples](#examples)
7. [Community and Support](#community-and-support)
8. [Reference](#reference)
9. [Further reading](#further-reading)

---

## Introduction

Learn the basics of Freenet and its purpose.

- [Introduction](introduction)

---

## Components {#components}

Explore the key components of Freenet:

- [Overview](components/overview): A high-level overview of Freenet's components.
- [Contracts](components/contracts): Details about contracts in Freenet.
- [Delegates](components/delegates): Explanation of delegates and their roles.
- [User Interfaces](components/ui): Information on available user interfaces.

---

## Architecture {#architecture}

Understand Freenet's architecture and how it works:

- [P2P Network](architecture/p2p-network): Explore the peer-to-peer network structure.
- [Intelligent Routing](architecture/irouting): Understand Freenet's intelligent routing mechanisms.
- [Transport](architecture/transport): Learn about the transport layer in Freenet.

---

## Developer Guide {#developer-guide}

Resources for building on Freenet:

- [Publish a Website](publish-a-website): Host a static website on Freenet -- no coding required.
- [Remote Access to a Node](remote-access): Safely reach your local node's API from another device
  (SSH tunnel, Tailscale).
- [Tutorial: Create an App](tutorial): Step-by-step guide to creating a decentralized app.
- [Contract Interfaces](contract-interface): The Rust contract-authoring API (`ContractInterface`).
  Full API on [docs.rs](https://docs.rs/freenet-stdlib).
- [Upgrading Contracts and Delegates](upgrading-contracts): Ship a new version without stranding
  users' state and secrets under the old key.
- [Manifest Format](manifest): Details about the `freenet.toml` configuration format.

---

## Client SDKs {#client-sdks}

Libraries for connecting a user interface to a Freenet node over WebSocket:

- [TypeScript SDK](typescript-sdk): The browser/Node.js client -- `@freenetorg/freenet-stdlib`.
  Recommended for most UIs.
- Rust client (Dioxus): use `freenet-stdlib` with the `net` feature; see
  [docs.rs](https://docs.rs/freenet-stdlib) and [River](https://github.com/freenet/river) for the
  reference implementation.

---

## Examples {#examples}

- [Example Apps](example-app): Canonical apps to learn from -- freenet-ping (minimal Rust), Raven
  (TypeScript + Vite), and River (Dioxus).
- [Antiflood Tokens](examples/antiflood-tokens)
- [Blind Trust Tokens](examples/blind-trust-tokens)

---

## Community and Support {#community-and-support}

Get involved with the Freenet community:

- [Community](community)

---

## Reference {#reference}

Additional resources and glossary:

- [Glossary](glossary)

---

## Further reading {#further-reading}

Deep-dive articles on the design principles behind Freenet's architecture:

- [Understanding Small World Networks](/build/manual/further-reading/small-world-networks/): the
  routing intuition behind the P2P network. How Freenet finds destinations in just a few hops
  without a central index.
- [Understanding Freenet's Delta-Sync](/build/manual/further-reading/delta-sync/): how shared state
  stays consistent across the network using mergeable, additive updates rather than full snapshots.

See [Further reading](/build/manual/further-reading/) for the full collection.
