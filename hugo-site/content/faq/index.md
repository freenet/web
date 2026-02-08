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
- [How does Freenet handle harmful content?](#how-does-freenet-handle-harmful-content)
- [What is the status of Freenet?](#what-is-the-status-of-freenet)
- [Can I follow Freenet on social media?](#can-i-follow-freenet-on-social-media)
- [How can I financially support Freenet development?](#how-can-i-financially-support-freenet-development)
- [Why does the Freenet project use and mention AI tools?](#why-does-the-freenet-project-use-and-mention-ai-tools)

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
[contracts](https://freenet.org/resources/manual/components/contracts/), and the values are also known as
the contract's **state**.

Like the web, most people will interact with Freenet through their web browser. Freenet provides a
local [HTTP proxy](https://freenet.org/resources/manual/components/ui/) that allows data such as a
[single-page application](https://en.wikipedia.org/wiki/Single-page_application) to be downloaded to
a web browser. This application can then connect to the Freenet peer through a
[websocket](https://en.wikipedia.org/wiki/WebSocket) connection and through this interact with the
Freenet network, including creating, reading, and modifying contracts and their state.

For a much more detailed explanation please see our
[user manual](https://freenet.org/resources/manual/introduction/).

# What is the project's history? {#what-is-the-projects-history}

Freenet was initially developed by Ian Clarke at the University of Edinburgh in 1999 as a
decentralized system for information storage and retrieval, offering users the ability to publish or
retrieve information anonymously. The 2001 paper,
["Freenet: A distributed anonymous information storage and retrieval system"](https://scholar.google.com/scholar?cluster=17926651926152536224&hl=en&as_sdt=0,44)
([PDF](/pdf/ADAISARS.pdf)), which describes the foundational work led by Ian Clarke and
contributions from a team of volunteers, has been cited over 3,700 times. This places it among the
most frequently cited computer science papers of its year, reflecting its broad influence on the
fields of distributed computing, peer-to-peer networking, and online anonymity.

In 2019, Ian began work on a successor to the original Freenet, which was internally known as
"Locutus." This project, a redesign from the ground up, incorporated lessons learned from the
original Freenet's development and operation, and adapted to today's challenges. In March 2023, the
original version of Freenet was separated into its
[own project](https://www.hyphanet.org/pages/about.html), and what was known as "Locutus" was
officially renamed to "Freenet."

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

# How does Freenet handle harmful content? {#how-does-freenet-handle-harmful-content}

Freenet will address harmful content with a decentralized reputation system, inspired by the
original Freenet's very effective
[Web of Trust plugin](https://github.com/hyphanet/plugin-WebOfTrust/blob/master/developer-documentation/core-developers-manual/OadSFfF-version1.2-non-print-edition.pdf)
but generalized and improved.

This system will allow users to collaboratively filter content, putting them in direct control to
shape their experience according to their own preferences. Freenet’s
[Ghost Keys](https://freenet.org/ghostkey) allow users to establish reputations privately, laying
the foundation for a decentralized reputation system that respects privacy.

No filtering approach is perfect, especially in centralized systems, but we think a decentralized
method can work as well or better.

# Can I follow Freenet on social media? {#can-i-follow-freenet-on-social-media}

Yes, you can follow [\@FreenetOrg](https://twitter.com/freenetorg) on Twitter/X or discuss
[r/freenet](https://www.reddit.com/r/Freenet/) on Reddit.

# How can I financially support Freenet development? {#how-can-i-financially-support-freenet-development}

Founded in 2001, Freenet is a 501c3 non-profit organization dedicated to the development and
propagation of technologies for open and democratic information distribution over the Internet. We
advocate for unrestricted exchange of intellectual, scientific, literary, social, artistic,
creative, human rights, and cultural expressions, free from interference by state, private, or
special interests.

You can [donate via credit card or cryptocurrency](/donate), or donate through our
[Ghost Key](/ghostkey/) mechanism to establish a verifiable cryptographic identity.

If you are in a position to make a larger contribution or grant please
{{< email-protect "gro.teneerf@nai" "email Ian" >}} or reach out to him on
[𝕏](https://x.com/sanity).

# Why does the Freenet project use and mention AI tools? {#why-does-the-freenet-project-use-and-mention-ai-tools}

Freenet does not require AI, depend on AI, or embed AI into the platform. Users and contributors can
use, build on, or participate in Freenet without interacting with AI in any form.

In practice, some of the Freenet codebase is written with extensive assistance from AI tools and then
reviewed, tested, and refined by humans. We use these tools because they dramatically increase
developer productivity, allowing a small team to build and iterate far more quickly than would
otherwise be possible.

For a project whose purpose is free communication, transparency and user agency matter more than
optimizing for any particular set of cultural or moral preferences. Freenet is about giving people
control over how they communicate and what tools they choose to use, not about enforcing or signaling
an ideology.
