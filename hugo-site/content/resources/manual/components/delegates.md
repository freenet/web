---
title: "Delegates"
date: 2025-04-13
draft: false
---

When you use an application that handles private keys—a wallet, a messaging app, anything with cryptographic identity—you're trusting that application with your secrets. Every library it imports, every dependency, every line of code has access to the same memory where your keys live. A single vulnerability anywhere in that stack can expose everything.

Delegates change this equation. A delegate holds your secrets and performs sensitive operations on your behalf, but the application itself never sees the secrets. The app asks the delegate to sign a message; the delegate returns a signature. The private key never crosses the boundary.

This is the core idea: **applications can use secrets without receiving them**.

## How delegates work

Delegates are software components that run inside the Freenet Core on a user's device. Other components—UIs, contracts, other delegates—can only interact with a delegate by sending messages. They cannot read its internal state directly.

This is the same security principle that classic encapsulation aimed for—data accessed only through controlled methods—but enforced by the platform across real trust boundaries. If a UI is buggy or compromised, the delegate can still refuse unsafe requests and keep secrets protected.

## How delegates fit into the architecture

Freenet applications typically involve three kinds of components. [Contracts](/resources/manual/components/contracts/) hold public, replicated state stored across the network. [User interfaces](/resources/manual/components/ui/) are web frontends that users interact with in their browser. Delegates are private agents that hold secrets and enforce rules for sensitive operations.

This separation keeps public state (contracts) and private state (delegates) distinct, while still enabling rich applications that span both.

## Message passing with known senders

Delegates communicate with contracts, UIs, and other delegates by passing messages, similar to the actor model. The Freenet Core ensures that when a delegate receives a message, it knows who sent it.

This allows delegates to implement policies: only respond to requests from an approved UI, only accept messages from specific delegates, only perform actions when a request is properly attributed. A delegate can make trust decisions based on the sender's identity and the content of the request.

## Encapsulation with teeth

A useful mental model is to think of a delegate as an object whose private fields truly cannot be bypassed.

Traditional encapsulation is mostly a convention. If code runs in the same process or environment, it can often read memory, call undocumented APIs, or exfiltrate local storage. Delegates provide encapsulation that the platform actually enforces: the delegate's private state is not directly accessible, the only way to interact is through its message interface, and the delegate can apply policy at the moment of use. This matters most for cryptographic keys and authorization logic.

## Example: signing without exporting a private key

A common use case is message signing. A UI wants to send an authenticated message, but instead of retrieving the user's private key, it asks the delegate: "Please sign this payload." The delegate checks who is asking and whether the request is allowed. If allowed, it produces a signature and returns it to the UI. The private key never leaves the delegate.

<img src="/delegate-signing.svg" alt="Delegate signing flow" style="max-width: 480px;">

## Implementation notes

Delegates are implemented in WebAssembly and conform to the `DelegateInterface` trait. At a high level, a delegate receives inbound messages, optionally consults secret state, and produces zero or more outbound messages as a response.

All durable delegate state must be stored using the secrets mechanism rather than in-process global state. This keeps private data under the delegate boundary and allows the core to manage it securely.

Beyond handling messages, delegates can create, read, and modify contracts; create other delegates; and ask the user questions for permission prompts or configuration.

## Use cases

A **key manager delegate** stores private keys and signs data on request, possibly prompting the user for permission. An **inbox delegate** monitors an inbox contract, downloads new messages, decrypts them, and stores them privately for UIs to display. A **contacts delegate** stores and retrieves contact information. An **alerts delegate** watches for events—like mentions in a discussion—and notifies the user.

Delegates can also synchronize with identical delegate instances running on other devices the user controls. With an appropriate shared secret, they communicate securely via Freenet and act as backups and replicas of each other.

For a real-world example, River's [chat delegate](https://github.com/freenet/river/tree/main/delegates/chat-delegate) stores per-room signing keys and signs messages, invitations, and room configurations on behalf of the UI—without ever exposing the keys.

## Comparison to service workers

Delegates share a pattern with browser service workers: self-contained modules running independently of the UI, performing tasks on the user's behalf. The key differences are scope and purpose. Delegates are not limited to the browser; they can communicate with other delegates locally and over Freenet; and they are designed specifically for private state, policy enforcement, and sensitive operations.
