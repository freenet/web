---
title: "Delegates"
date: 2025-04-13
draft: false
---

# Delegates

Delegates are software components that run inside the Freenet Core on a user's device. They act on the user's behalf while keeping private data private.

The simplest way to remember what a delegate is:

A delegate lets applications use secrets without ever receiving the secrets.

This is the same security principle that classic encapsulation aimed for (data is only accessed through controlled methods), but enforced by the platform across real trust boundaries. Other components can only interact with a delegate by sending messages; they cannot read its internal state directly.

## What problem delegates solve

In most systems, private keys, tokens, and personal state end up spread across many layers: UI code, application code, plugins, libraries, and storage. Even when data is "local," it is often accessible to any code running in the same environment.

Delegates reduce the attack surface by centralizing sensitive state and operations behind a strict interface:

- Secrets are stored and used inside the delegate.
- Callers ask the delegate to perform actions (for example: sign, decrypt, authorize).
- The delegate returns results (for example: a signature), not the secret itself.

If a UI or application is buggy or compromised, the delegate can still refuse unsafe requests and can avoid exposing long-lived secrets.

## How delegates fit into the Freenet architecture

Freenet applications typically involve three kinds of components:

- Contracts: public, replicated application state stored and updated via the network.
- User interfaces (UIs): web frontends that users interact with.
- Delegates: private agents that hold secrets and enforce rules for sensitive operations.

This separation makes it natural to keep public state (contracts) and private state (delegates) distinct, while still enabling rich applications.

## Communication model: message passing with known senders

Delegates communicate with contracts, UIs, and other delegates by passing messages (similar to the actor model). The Freenet Core ensures that when a delegate receives a message, it can determine who sent it.

This allows delegates to implement policies such as:

- only respond to requests from an approved UI,
- only accept messages from specific delegates,
- only perform actions when a request is properly attributed.

In other words, a delegate can make trust decisions based on the sender identity and the request.

## Delegates as enforced encapsulation

A useful mental model is to think of a delegate as an "object" whose private fields cannot be bypassed.

Traditional encapsulation is mostly a convention: if code runs in the same process or environment, it often can read memory, call undocumented APIs, or exfiltrate local storage.

Delegates provide encapsulation with teeth:

- the delegate's private state is not directly accessible,
- the only way to interact is through the delegate's message interface,
- the delegate can apply policy at the moment of use.

This is especially important for cryptographic keys and authorization logic.

## Example: signing without exporting a private key

A common use case is message signing:

1. A UI wants to send an authenticated message.
2. Instead of retrieving the user's private key, the UI asks the delegate: "Please sign this payload."
3. The delegate checks who is asking and whether the request is allowed.
4. If allowed, the delegate produces a signature and returns it to the UI.

The private key never leaves the delegate.

## Implementation notes for developers

Delegates are implemented in WebAssembly and conform to the `DelegateInterface` trait.

At a high level, a delegate:

- receives inbound messages,
- optionally consults secret state,
- produces zero or more outbound messages as a response.

All durable delegate state must be stored using the secrets mechanism rather than in-process global state. This keeps private data under the delegate boundary and allows the core to manage it securely.

Delegates can also:

- create, read, and modify contracts,
- create other delegates,
- send and receive messages with other components,
- ask the user questions and receive answers (for permission prompts or configuration).

## Delegate use cases

Delegates can be used for many roles, including:

- Key manager delegate: stores private keys and signs
