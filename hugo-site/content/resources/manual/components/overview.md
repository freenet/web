---
title: "Overview"
date: 2025-04-13
draft: false
weight: 1
---


## Components of Decentralized Software

Delegates, contracts, and user interfaces (UIs) each serve distinct roles in the
Freenet ecosystem. [Contracts](/manual/components/contracts) control public data, or "shared
state". [Delegates](/manual/components/delegates) act as the user's agent and can store private
data on the user's behalf, while [User Interfaces](/manual/components/ui) provide an interface
between these and the user through a web browser. UIs are distributed through
the P2P network via contracts.

![Architectural Primitives Diagram](/components.svg)

## Freenet Core

The Freenet Core is the software that enables a user's computer to connect to
the Freenet network. Its primary functions are:

- Providing a user-friendly interface to access Freenet via a web browser
- Host the user's [delegates](/manual/components/delegates) and the private data they store
- Host [contracts](/manual/components/contracts) and their associated data on behalf of the
  network
- Manage communication between contracts, delegates, and UI components

Built with Rust, the core is designed to be compact (ideally under 5 MB),
efficient, and capable of running on a variety of devices such as smartphones,
desktop computers, and embedded devices.
