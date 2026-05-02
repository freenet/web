---
title: "Build on Freenet"
date: 2024-06-11T00:00:00Z
draft: false
aliases:
  - /dev/
  - /dev/platform/
  - /dev/apps/
---

## Getting Started

Build decentralized applications using familiar tools (Rust, TypeScript) and deploy them to a global
peer-to-peer network with no servers to maintain.

- [Tutorial](/build/manual/tutorial/) - Build your first Freenet app
- [Manual](/build/manual/) - Architecture, components, and reference

## AI-Assisted Development

Install the [freenet-dapp-builder](https://github.com/freenet/freenet-agent-skills/tree/main/skills/dapp-builder) skill for Claude Code:

```bash
/plugin marketplace add freenet/freenet-agent-skills
/plugin install freenet-dapp-builder
```

This skill guides you through building contracts, delegates, and UI for Freenet apps.

## Developer Tools

[![GitHub](https://img.shields.io/badge/GitHub-freenet--core-blue?logo=github)](https://github.com/freenet/freenet-core)
[![Crates.io](https://img.shields.io/badge/Crates.io-freenet-orange?logo=rust)](https://crates.io/crates/freenet)

*   **[freenet-scaffold](https://github.com/freenet/freenet-scaffold)**: A Rust utility crate that simplifies the development of Freenet contracts by providing tools to implement efficient, mergeable state synchronization.

*   **[ghostkeys](https://github.com/freenet/ghostkeys)**: A Freenet delegate for managing [ghost key](/ghostkey/) identities, enabling trust verification without revealing identity through blind-signed cryptographic certificates.

## Example Apps

- [freenet-ping](https://github.com/freenet/freenet-core/tree/main/apps/freenet-ping) - a simple
  example demonstrating how to build a Freenet app
- [River](https://github.com/freenet/river) - decentralized group chat app built on Freenet (in
  development)
- [Delta](https://github.com/freenet/delta) - decentralized publishing app built on Freenet
- [freenet-git](https://github.com/freenet/freenet-git) - decentralized Git collaboration on
  Freenet (in development)
- [ghostkeys](https://github.com/freenet/ghostkeys) - Freenet delegate for managing
  [ghost key](/ghostkey/) identities

## Recently Merged Features

{{< merged-pull-requests >}}
