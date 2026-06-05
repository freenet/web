---
title: "Build on Freenet"
date: 2024-06-11T00:00:00Z
draft: false
aliases:
  - /dev/
  - /dev/platform/
---

## Getting Started

Build decentralized applications using familiar tools (Rust, TypeScript) and deploy them to a global
peer-to-peer network with no servers to maintain.

- [Tutorial](/build/manual/tutorial/) - Build your first Freenet app
- [Manual](/build/manual/) - Architecture, components, and reference
- [TypeScript SDK](/build/manual/typescript-sdk/) - Connect a browser or Node.js UI to a node with `@freenetorg/freenet-stdlib`

## AI-Assisted Development

Install the [dapp-builder](https://github.com/freenet/freenet-agent-skills/tree/main/skills/dapp-builder) skill for Claude Code:

```bash
/plugin marketplace add freenet/freenet-agent-skills
/plugin install freenet@freenet-agent-skills
```

This installs the `freenet` plugin, whose `dapp-builder` skill guides you through building
contracts, delegates, and UI for Freenet apps. The plugin also bundles `local-dev` and other
development skills.

## Developer Tools

[![GitHub](https://img.shields.io/badge/GitHub-freenet--core-blue?logo=github)](https://github.com/freenet/freenet-core)
[![Crates.io](https://img.shields.io/badge/Crates.io-freenet-orange?logo=rust)](https://crates.io/crates/freenet)

*   **[freenet-scaffold](https://github.com/freenet/freenet-scaffold)**: A Rust utility crate that simplifies the development of Freenet contracts by providing tools to implement efficient, mergeable state synchronization.

*   **[ghostkeys](https://github.com/freenet/ghostkeys)**: A Freenet delegate for managing [ghost key](/ghostkey/) identities, enabling trust verification without revealing identity through blind-signed cryptographic certificates.

## Example Apps {#example-apps}

See [Apps & Ecosystem](/apps/) for a current list of applications and components built on Freenet,
including links to the source code for each project.

A minimal worked example for developers:
[freenet-ping](https://github.com/freenet/freenet-core/tree/main/apps/freenet-ping): the smallest
useful Freenet app, demonstrating the contract / delegate / UI loop end-to-end.

## Recently Merged Features

{{< merged-pull-requests >}}
