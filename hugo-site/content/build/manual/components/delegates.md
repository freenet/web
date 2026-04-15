---
title: "Delegates"
date: 2025-04-13
draft: false
aliases:
  - /resources/manual/components/delegates/
---

When you use an application that handles private keys (a wallet, a messaging app, anything with cryptographic identity), you're trusting that application with your secrets. Every library it imports, every dependency, every line of code has access to the same memory where your keys live. A single vulnerability anywhere in that stack can expose everything.

Delegates change this equation. A delegate holds your secrets and performs sensitive operations on your behalf, but the application itself never sees the secrets. The app asks the delegate to sign a message; the delegate returns a signature. The private key never crosses the boundary.

This is the core idea: **applications can use secrets without receiving them**.

## How delegates work

Delegates are software components that run inside the Freenet Core on a user's device. Other components (UIs, contracts, other delegates) can only interact with a delegate by sending messages. They cannot read its internal state directly.

This is the same security principle that classic encapsulation aimed for (data accessed only through controlled methods), but enforced by the platform across real trust boundaries. If a UI is buggy or compromised, the delegate can still refuse unsafe requests and keep secrets protected.

## How delegates fit into the architecture

Freenet applications typically involve three kinds of components. [Contracts](/build/manual/components/contracts/) hold public, replicated state stored across the network. [User interfaces](/build/manual/components/ui/) are web frontends that users interact with in their browser. Delegates are private agents that hold secrets and enforce rules for sensitive operations.

This separation keeps public state (contracts) and private state (delegates) distinct, while still enabling rich applications that span both.

## Message passing with known senders

Delegates communicate with contracts, UIs, and other delegates by passing messages, similar to the actor model. The Freenet Core ensures that when a delegate receives a message, it knows who sent it.

This allows delegates to implement policies: only respond to requests from an approved UI, only accept messages from specific delegates, only perform actions when a request is properly attributed. A delegate can make trust decisions based on the sender's identity and the content of the request.

## Encapsulation with teeth

A useful mental model is to think of a delegate as an object whose private fields truly cannot be bypassed.

Traditional encapsulation is mostly a convention. If code runs in the same process or environment, it can often read memory, call undocumented APIs, or exfiltrate local storage. Delegates provide encapsulation that the platform actually enforces: the delegate's private state is not directly accessible, the only way to interact is through its message interface, and the delegate can apply policy at the moment of use. This matters most for cryptographic keys and authorization logic.

## Example: signing without exporting a private key

A common use case is message signing. A UI wants to send an authenticated message, but instead of retrieving the user's private key, it asks the delegate: "Please sign this payload." The delegate checks who is asking and whether the request is allowed. If allowed, it produces a signature and returns it to the UI. The private key never leaves the delegate.

<img src="/delegate-signing.svg" alt="Delegate signing flow" style="max-width: 480px; display: block; margin: 1.5rem auto;">

## Requesting user consent

A delegate MAY defer a decision to the user. When a delegate receives a request whose disposition depends on user intent, for example the signing of an unfamiliar payload, or an action initiated by a caller the delegate does not yet trust, it emits a `RequestUserInput` message containing a prompt string and an ordered set of response options.

Freenet Core routes the prompt to the shell page that hosts Freenet applications in the browser. The shell page is trusted and runs outside the sandboxed application iframe. It renders the prompt as an overlay above the iframe, displaying the delegate's message, the offered options, the identity of the delegate, and the attested identity of the caller (either a UI contract or another delegate). When the user selects an option, the selection is delivered to the delegate as an `InboundDelegateMsg::UserResponse`. The delegate then resumes processing and determines the outcome.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="/img/delegate-permission-prompt-dark.png">
  <img src="/img/delegate-permission-prompt-light.png" alt="A Permission Request overlay. The card shows the delegate's message ('A Freenet application (DLog47hE...) is requesting access to your ghostkey identity (oiQaxxSwKF8)'), three buttons (Allow Once, Always Allow, Deny), the attested delegate identity, and an auto-deny countdown." style="max-width: 520px; display: block; margin: 1.5rem auto;">
</picture>

The screenshot above shows a ghostkey delegate prompting the user. The card is drawn by the shell page on top of the application iframe; the delegate's own message appears in a quoted block, the caller and delegate identities are supplied by Freenet Core, and an auto-deny countdown ensures the request cannot hang indefinitely.

The following properties hold:

1. The prompt is initiated by the delegate. An application cannot construct, suppress, or answer a prompt on the user's behalf; it can only submit requests to the delegate, which decides whether to involve the user.

2. The prompt is rendered outside the application sandbox. The overlay is drawn by the shell page in its own DOM. A compromised or hostile UI cannot read, obscure, or synthesize input to it.

3. Caller identity is attested by the platform. The delegate and caller identifiers presented to the user are supplied by Freenet Core, not by the caller. Delegates MAY apply distinct policies to distinct callers on this basis.

4. Absence of a response is treated as denial. If the prompt is not answered within the timeout, the request is denied and the delegate is notified accordingly. A missed prompt cannot result in an approval.

Together, these properties make the delegate a consent boundary rather than an advisory one. A key-manager delegate, for instance, may sign routine messages for a caller it has previously accepted and prompt the user the first time an unrecognised caller requests a signature; because the caller cannot bypass the delegate, the user's answer is authoritative.

## Implementation notes

Delegates are implemented in WebAssembly and conform to the `DelegateInterface` trait. At a high level, a delegate receives inbound messages, optionally consults secret state, and produces zero or more outbound messages as a response.

All durable delegate state must be stored using the secrets mechanism rather than in-process global state. This keeps private data under the delegate boundary and allows the core to manage it securely.

Beyond handling messages, delegates can create, read, and modify contracts; create other delegates; and prompt the user for consent as described above.

## Use cases

A **key manager delegate** stores private keys and signs data on request, possibly prompting the user for permission. An **inbox delegate** monitors an inbox contract, downloads new messages, decrypts them, and stores them privately for UIs to display. A **contacts delegate** stores and retrieves contact information. An **alerts delegate** watches for events (like mentions in a discussion) and notifies the user.

Delegates can also synchronize with identical delegate instances running on other devices the user controls. With an appropriate shared secret, they communicate securely via Freenet and act as backups and replicas of each other.

For a real-world example, River's [chat delegate](https://github.com/freenet/river/tree/main/delegates/chat-delegate) stores per-room signing keys and signs messages, invitations, and room configurations on behalf of the UI, without ever exposing the keys.

## Comparison to service workers

Delegates share a pattern with browser service workers: self-contained modules running independently of the UI, performing tasks on the user's behalf. The key differences are scope and purpose. Delegates are not limited to the browser; they can communicate with other delegates locally and over Freenet; and they are designed specifically for private state, policy enforcement, and sensitive operations.
