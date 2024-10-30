---
title: "Frequently Asked Questions"
date: 2024-06-24
draft: false
---

{{< toc >}}

- [What is Freenet?](#what-is-freenet)
- [How does Freenet work?](#how-does-freenet-work)
- [What is the project's history?](#what-is-the-projects-history)
- [How do the previous and current versions of Freenet differ?](#how-do-the-previous-and-current-versions-of-freenet-differ)
- [Will the new Freenet be backwards compatible with the old Freenet?](#will-the-new-freenet-be-backwards-compatible-with-the-old-freenet)
- [Why was Freenet rearchitected and rebranded?](#why-was-freenet-rearchitected-and-rebranded)
- [How does Freenet compare to other decentralized systems?](#how-does-freenet-compare-to-other-decentralized-systems)
- [Who is behind Freenet?](#who-is-behind-freenet)
- [How does Freenet handle undesirable or harmful content, such as spam or illegal material?](#how-does-freenet-handle-undesirable-or-harmful-content-such-as-spam-or-illegal-material)
- [What is the status of Freenet?](#what-is-the-status-of-freenet)
- [Can I follow Freenet on social media?](#can-i-follow-freenet-on-social-media)
- [How can I financially support Freenet development?](#how-can-i-financially-support-freenet-development)

Freenet is a fully decentralized, peer-to-peer network and a drop-in replacement for the world wide
web. It operates as a global shared computer, providing a platform for sophisticated decentralized
software systems. Freenet allows developers to create decentralized alternatives to centralized
services, including messaging, social media, email, and e-commerce. It's designed for simplicity and
flexibility and can be used seamlessly through your web browser. The platform's user-friendly
decentralized applications are scalable, interoperable, and secured with cryptography.

# How does Freenet work? {#how-does-freenet-work}

Freenet is a global key-value store that relies on
[small world routing](https://en.wikipedia.org/wiki/Small-world_routing) for decentralization and
scalability. Keys in this key-value store are
[WebAssembly](https://en.wikipedia.org/wiki/WebAssembly) code which specify:

- When is a value permitted under this key?
  - eg. verify that the value is cryptographically signed with a particular public key
- Under what circumstances may the value be modified
  - eg. modifications must be signed
- How can the value be efficiently synchronized between peers in the network

These webassembly keys are also known as
[contracts](https://docs.freenet.org/components/contracts.html), and the values are also known as
the contract's **state**.

Like the web, most people will interact with Freenet through their web browser. Freenet provides a
local [HTTP proxy](https://docs.freenet.org/components/ui.html) that allows data such as a
[single-page application](https://en.wikipedia.org/wiki/Single-page_application) to be downloaded to
a web browser. This application can then connect to the Freenet peer through a
[websocket](https://en.wikipedia.org/wiki/WebSocket) connection and through this interact with the
Freenet network, including creating, reading, and modifying contracts and their state.

For a much more detailed explanation please see our
[user manual](https://docs.freenet.org/introduction.html).

# What is the project's history? {#what-is-the-projects-history}

Freenet was initially developed by Ian Clarke at the University of Edinburgh in 1999 as a
decentralized system for information storage and retrieval, offering users the ability to publish or
retrieve information anonymously.

In 2019, Ian began work on a successor to the original Freenet, which was internally known as
"Locutus." This project, a redesign from the ground up, incorporated lessons learned from the
original Freenet's development and operation, and adapted to today's challenges. In March 2023, the
original version of Freenet was separated into its
[own project](https://www.hyphanet.org/pages/about.html), and what was known as "Locutus" was
officially branded as "Freenet."

# How do the previous and current versions of Freenet differ? {#how-do-the-previous-and-current-versions-of-freenet-differ}

The previous and current versions of Freenet have several key differences:

- Functionality: The previous version was analogous to a decentralized hard drive, while the current
  version is analogous to a full decentralized computer.

- Real-time Interaction: The current version allows users to subscribe to data and be notified
  immediately if it changes. This is essential for systems like instant messaging or group chat.

- Programming Language: Unlike the previous version, which was developed in Java, the current
  Freenet is implemented in Rust. This allows for better efficiency and integration into a wide
  variety of platforms (Windows, Mac, Android, MacOS, etc).

- Transparency: The current version is a drop-in replacement for the world wide web and is just as
  easy to use.

- Anonymity: While the previous version was designed with a focus on anonymity, the current version
  does not offer built-in anonymity but allows for a choice of anonymizing systems to be layered on
  top.

# Will the new Freenet be backwards compatible with the old Freenet? {#will-the-new-freenet-be-backwards-compatible-with-the-old-freenet}

No, the new Freenet is a fundamental redesign making backwards compatibility impractical.

# Why was Freenet rearchitected and rebranded? {#why-was-freenet-rearchitected-and-rebranded}

In 2019, Ian began developing a successor to the original Freenet, internally named "Locutus." This
redesign was a ground-up reimagining, incorporating lessons learned from the original Freenet and
addressing modern challenges. The original Freenet, although groundbreaking, was built for an
earlier era.

This isn't the first time Freenet has undergone significant changes. Around 2005, we transitioned
from version 0.5 to 0.7, which was a complete rewrite introducing "friend-to-friend" networking.

In March 2023, the original Freenet (developed from 2005 onwards) was spun off into an independent
project called "Hyphanet" under its existing maintainers. Concurrently, "Locutus" was rebranded as
"Freenet," also known as "Freenet 2023," to signal this new direction and focus. The rearchitected
Freenet is faster, more flexible, and better equipped to offer a robust, decentralized alternative
to the increasingly centralized web.

To ease the transition the old freenetproject.org domain was redirected to hyphanet's website, while
the recently acquired freenet.org domain was used for the new architecture.

It is important to note that the maintainers of the original Freenet did not agree with the decision
to rearchitect and rebrand. However, as the architect of the Freenet Project, and after over a year
of debate, Ian felt this was the necessary path forward to ensure the project's continued relevance
and success in a world far different than when he designed the previous architecture.

# How does Freenet compare to other decentralized systems? {#how-does-freenet-compare-to-other-decentralized-systems}

**1. Freenet is a Complete Solution**

Freenet functions as an end-to-end operating system for decentralized apps. Similar to how you
install a web browser once and gain access to applications like Gmail, Facebook, and Reddit without
installing additional software, Freenet provides seamless access to a wide range of decentralized
applications directly within your browser.

With Freenet, you can:

- Discover apps through a decentralized search engine.
- Obtain apps through Freenet.
- Use apps entirely on Freenet.

Additionally, you don't have to use Freenet through a browser. The "Freenet core" is small (<10MB)
and can be easily embedded in other software, which can then communicate with the Freenet core over
an HTTP/WebSocket API.

In contrast, most other systems function more like toolkits for building decentralized apps—akin to
providing a crankshaft rather than a complete car. Developers use them to integrate peer-to-peer
functionality into existing applications, often requiring extra components and setup for end-users.

**2. Unique Architectural Approach**

Freenet operates as a global key-value store where keys correspond to WebAssembly (Wasm) code,
referred to as "contracts." These contracts define the properties and behavior of the associated
values (or "state"). Specifically, they govern:

1. **Validity:** Is the value valid for this key? For instance, the contract might verify that the
   data is signed by a specific public key.
2. **Modification Rules:** Under what circumstances can the value be modified? A contract might
   stipulate that any modification must be signed by a specific key.
3. **Efficient Synchronization:** How to efficiently synchronize values between peers? Freenet
   ensures eventual consistency by treating values as commutative monoids, allowing updates in any
   order while still producing the same result.

This unique architectural approach makes Freenet a powerful, general-purpose platform for building
decentralized systems that are scalable and interoperable by default.

**3. Truly Decentralized, not Federated**

Freenet is fully decentralized, unlike federated systems where multiple entities control different
servers. Moving from a centralized system to a federated one is like going from a dictatorship to
feudalism—an improvement, but users still have to trust the system operators. Freenet eliminates
this need for trust by distributing control, ensuring the sovereignty of each user.

# Who is behind Freenet? {#who-is-behind-freenet}

Freenet was started by Ian Clarke in 1999 and grew out of his undergraduate [paper](/pdf/DDISRS.pdf)
"A Distributed Decentralized Information Storage and Retrieval System."

To further the goals of the project, Ian Clarke and Steven Starr co-founded The Freenet Project, a
501c3 non-profit in 2001.

In 2024, The Freenet Project non-profit board of directors consists of Ian Clarke, Steven Starr, and
Michael Grube. Ian and Steven are actively involved in the day-to-day operations of the project.

# How does Freenet handle undesirable or harmful content, such as spam or illegal material? {#how-does-freenet-handle-undesirable-or-harmful-content-such-as-spam-or-illegal-material}

To address undesirable or harmful content like spam or illegal material, Freenet is building a
decentralized reputation system inspired by the original
[Web of Trust plugin](https://github.com/hyphanet/plugin-WebOfTrust/blob/master/developer-documentation/core-developers-manual/OadSFfF-version1.2-non-print-edition.pdf),
but with key improvements.

This system enables users to collaboratively filter harmful content, putting control directly in
their hands to shape their experience according to their own preferences.

Freenet’s [Ghost Keys](https://freenet.org/ghostkey) allow users to establish reputations privately,
a foundational step toward a decentralized reputation system that respects privacy.

It’s also important to note that Freenet serves as a communication medium rather than a storage
medium and is not designed for distributing large files like video.

While no filtering solution is perfect, especially with today’s centralized systems, we believe a
decentralized approach offers a more effective, privacy-respecting path forward.

# What is the status of Freenet? {#what-is-the-status-of-freenet}

As of September 2024, we are very close to getting the network up; see our
[news page](https://freenet.org/news) for regular status updates. In the meantime you can already
[experiment](https://docs.freenet.org/tutorial.html) with building a decentralized app to test on
your own computer.

# Can I follow Freenet on social media? {#can-i-follow-freenet-on-social-media}

Yes, you can follow [\@FreenetOrg](https://twitter.com/freenetorg) on Twitter/X or discuss
[r/freenet](https://www.reddit.com/r/Freenet/) on Reddit.

# How can I financially support Freenet development? {#how-can-i-financially-support-freenet-development}

Founded in 2001, Freenet is a 501c3 non-profit organization dedicated to the development and
propagation of technologies for open and democratic information distribution over the Internet. We
advocate for unrestricted exchange of intellectual, scientific, literary, social, artistic,
creative, human rights, and cultural expressions, free from interference by state, private, or
special interests.

#### Donate via PayPal or Credit Card

[![Donate with
PayPal](https://www.paypalobjects.com/en_US/i/btn/btn_donate_SM.gif)](https://www.paypal.com/donate?hosted_button_id=EQ9E7DPHB6ETY)

#### Donate via Cryptocurrency

Freenet is **not** a cryptocurrency, but we do accept cryptocurrency donations. For large donations
(over \$5,000) please contact us before sending. For smaller donations, please use the following
wallets:

| Cryptocurrency | Address                                        |
| -------------- | ---------------------------------------------- |
| Bitcoin        | `3M3fbA7RDYdvYeaoR69cDCtVJqEodo9vth`           |
| Zcash          | `t1VHw1PHgzvMqEEd31ZBt3Vyy2UrG4J8utB`          |
| Ethereum       | `0x79158A5Dbd9C0737CB2741181we7BD2759f5b9a9Ae` |
