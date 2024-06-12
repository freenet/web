---
title: "How does Freenet work?"
---

# How does Freenet work?

Freenet is a global key-value store that relies on [small world
routing](https://en.wikipedia.org/wiki/Small-world_routing) for
decentralization and scalability. Keys in this key-value store are
[WebAssembly](https://en.wikipedia.org/wiki/WebAssembly) code which
specify:

-   When is a value permitted under this key?
    -   eg. verify that the value is cryptographically signed with a
        particular public key
-   Under what circumstances may the value be modified
    -   eg. modifications must be signed
-   How can the value be efficiently synchronized between peers in the
    network

These webassembly keys are also known as
[contracts](https://docs.freenet.org/components/contracts.html), and the
values are also known as the contract\'s **state**.

Like the web, most people will interact with Freenet through their web
browser. Freenet provides a local [HTTP
proxy](https://docs.freenet.org/components/ui.html) that allows data
such as a [single-page
application](https://en.wikipedia.org/wiki/Single-page_application) to
be downloaded to a web browser. This application can then connect to the
Freenet peer through a
[websocket](https://en.wikipedia.org/wiki/WebSocket) connection and
through this interact with the Freenet network, including creating,
reading, and modifying contracts and their state.

For a much more detailed explanation please see our [user
manual](https://docs.freenet.org/introduction.html).
