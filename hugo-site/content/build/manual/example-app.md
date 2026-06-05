---
title: "Example Apps"
date: 2025-04-13
draft: false
aliases:
  - /resources/manual/example-app/
---

The fastest way to learn how a Freenet application fits together (contract, optional delegate, and
UI) is to read a working one. These are the canonical examples, from smallest to most complete:

- **[freenet-ping](https://github.com/freenet/freenet-core/tree/main/apps/freenet-ping)**: the
  smallest useful app. A Rust contract plus a small CLI that publishes, updates, and subscribes.
  Best for understanding the contract lifecycle without any UI.
- **[Raven](https://github.com/freenet/raven)**: a decentralized microblogging app and a complete
  **TypeScript + Vite** frontend built on the
  [`@freenetorg/freenet-stdlib`](https://www.npmjs.com/package/@freenetorg/freenet-stdlib) SDK. The
  reference for a browser frontend.
- **[River](https://github.com/freenet/river)**: a production decentralized group-chat app with a
  **Dioxus** (Rust → WebAssembly) UI and a chat delegate. The reference for the patterns in the
  [tutorial](/build/manual/tutorial/).

For a step-by-step walkthrough of building your own app, follow the
[tutorial](/build/manual/tutorial/).

## Prerequisites

```bash
# Rust toolchain + the WebAssembly target
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown

# The Freenet node and the fdev developer tool
git clone https://github.com/freenet/freenet-core.git
cd freenet-core
cargo install --path crates/core   # `freenet` (the node)
cargo install --path crates/fdev   # `fdev` (publish/dev tooling)
```

A TypeScript UI (such as Raven) additionally needs [npm](https://www.npmjs.com/); the SDK is pulled
in with `npm install @freenetorg/freenet-stdlib`.

## Running freenet-ping

`freenet-ping` ships a small Makefile. From `apps/freenet-ping` in the freenet-core checkout:

```bash
# Start a local node in another terminal first:
freenet local

# Build the contract and CLI
make -f run-ping.mk build

# Run against the local node's WebSocket API port
make -f run-ping.mk run WS_PORT=7509
```

The app generates a random name and sends an update every second, logging the responses it receives
from the contract. See the
[freenet-ping README](https://github.com/freenet/freenet-core/tree/main/apps/freenet-ping) for the
full set of options.

## Publishing a website

To wrap a static site or a built UI in a signed web container and serve it over Freenet, see
[Publish a Website](/build/manual/publish-a-website/).
