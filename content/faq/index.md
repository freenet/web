+++
name = "FAQ"
+++

- [What is Freenet?](#what-is-freenet)
- [How does Freenet work?](#how-does-freenet-work)
- [What is the project's history?](#what-is-the-projects-history)
- [How do the previous and current versions of Freenet differ?](#how-do-the-previous-and-current-versions-of-freenet-differ)
- [Why was Freenet rearchitected and rebranded?](#why-was-freenet-rearchitected-and-rebranded)
- [What are the key components of Freenet's architecture?](#what-are-the-key-components-of-freenets-architecture)
- [Who is behind Freenet?](#who-is-behind-freenet)
- [What is the status of Freenet?](#what-is-the-status-of-freenet)
- [Can anyone use Freenet?](#can-anyone-use-freenet)
- [Can I follow Freenet on social media?](#can-i-follow-freenet-on-social-media)
- [How can I financially support Freenet development?](#how-can-i-financially-support-freenet-development)

## <a name="what-is-freenet"></a>What is Freenet?

Freenet is a fully decentralized, peer-to-peer network and a drop-in
replacement for the world wide web. It operates as a global shared
computer, providing a platform for sophisticated decentralized software
systems. Freenet allows developers to create decentralized alternatives
to centralized services, including messaging, social media, email, and
e-commerce. It\'s designed for simplicity and flexibility and can be
used seamlessly through your web browser. The platform\'s user-friendly
decentralized applications are scalable, interoperable, and secured with
cryptography.

## <a name="how-does-freenet-work"></a>How does Freenet work?

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

## <a name="what-is-the-projects-history"></a>What is the project's history?

Freenet was initially developed by Ian Clarke at the University of
Edinburgh in 1999 as a decentralized system for information storage and
retrieval, offering users the ability to publish or retrieve information
anonymously.

In 2019, Ian began work on a successor to the original Freenet, which
was internally known as \"Locutus.\" This project, a redesign from the
ground up, incorporated lessons learned from the original Freenet\'s
development and operation, and adapted to today\'s challenges. In March
2023, the original version of Freenet was separated into its [own
project](https://www.hyphanet.org/pages/about.html), and what was known
as \"Locutus\" was officially branded as \"Freenet.\"

## <a name="how-do-the-previous-and-current-versions-of-freenet-differ"></a>How do the previous and current versions of Freenet differ?

The previous and current versions of Freenet have several key
differences:

-   Functionality: The previous version was analogous to a decentralized
    hard drive, while the current version is analogous to a full
    decentralized computer.

-   Real-time Interaction: The current version allows users to subscribe
    to data and be notified immediately if it changes. This is essential
    for systems like instant messaging or group chat.

-   Programming Language: Unlike the previous version, which was
    developed in Java, the current Freenet is implemented in Rust. This
    allows for better efficiency and integration into a wide variety of
    platforms (Windows, Mac, Android, MacOS, etc).

-   Transparency: The current version is a drop-in replacement for the
    world wide web and is just as easy to use.

-   Anonymity: While the previous version was designed with a focus on
    anonymity, the current version does not offer built-in anonymity but
    allows for a choice of anonymizing systems to be layered on top.

## <a name="why-was-freenet-rearchitected-and-rebranded"></a>Why was Freenet rearchitected and rebranded?

In 2019, Ian began developing a successor to the original Freenet,
internally named \"Locutus.\" This redesign was a ground-up reimagining,
incorporating lessons learned from the original Freenet and addressing
modern challenges. The original Freenet, although groundbreaking, was
built for an earlier era.

This isn\'t the first time Freenet has undergone significant changes.
Around 2005, we transitioned from version 0.5 to 0.7, which was a
complete rewrite introducing \"friend-to-friend\" networking.

In March 2023, the original Freenet (developed from 2005 onwards) was
spun off into an independent project called \"Hyphanet\" under its
existing maintainers. Concurrently, \"Locutus\" was rebranded as
\"Freenet,\" also known as \"Freenet 2023,\" to signal this new
direction and focus. The rearchitected Freenet is faster, more flexible,
and better equipped to offer a robust, decentralized alternative to the
increasingly centralized web.

It is important to note that the maintainers of the original Freenet did
not agree with the decision to rearchitect and rebrand. However, as the
architect of the Freenet Project, and after over a year of debate, Ian
felt this was the necessary path forward to ensure the project\'s
continued relevance and success in a world very different than when he
designed the previous architecture.

## <a name="what-are-the-key-components-of-freenets-architecture"></a>What are the key components of Freenet's architecture?

Delegates, contracts, and user interfaces (UIs) each serve distinct
roles in the Freenet ecosystem. Contracts control public data, or
\"shared state.\" Delegates act as the user\'s agent and can store
private data on the user\'s behalf, while UIs provide an interface
between these and the user through a web browser. See the [user
manual](https://docs.freenet.org/components/overview.html) for more
detail.

## <a name="who-is-behind-freenet"></a>Who is behind Freenet?

Freenet was started by Ian Clarke in 1999 and grew out of his
undergraduate
[paper](https://cs.baylor.edu/~donahoo/classes/5321/papers/C99.pdf) \"A
Distributed Decentralized Information Storage and Retrieval System.\"

To further the goals of the project, Ian Clarke and Steven Starr
co-founded The Freenet 501c3 non-profit in 2001.

In 2024, the Freenet non-profit board of directors consists of Ian
Clarke, Steven Starr, and Michael Grube, with Ian serving as President
and Steven as Chief Strategy Officer. Along with Ian, the development
team consists of Nacho Duart and Hector Alberto Santos Rodriguez.

## <a name="what-is-the-status-of-freenet"></a>What is the status of Freenet?

As of June 2024, we are very close to getting the network up; see our
[blog](https://freenet.org/blog) for regular status updates. In the
meantime you can already
[experiment](https://docs.freenet.org/tutorial.html) with building a
decentralized app to test on your own computer.

## <a name="can-anyone-use-freenet"></a>Can anyone use Freenet?

While Freenet is designed to be accessible to most users, approximately
10-20% of users might experience connectivity issues due to being behind
symmetric NATs or restrictive firewalls. These network configurations,
often implemented by ISPs, can prevent direct peer-to-peer connections,
which are essential for Freenet\'s decentralized network. Users behind
such configurations might also face difficulties with other applications
requiring low-latency connections, such as multiplayer games and VoIP
services. We recommend choosing ISPs that offer less restrictive NAT
configurations to ensure a better overall internet experience and
seamless use of Freenet.

## <a name="can-i-follow-freenet-on-social-media"></a>Can I follow Freenet on social media?

Yes, you can follow [\@FreenetOrg](https://twitter.com/freenetorg) on
Twitter/X or discuss [r/freenet](https://www.reddit.com/r/Freenet/) on
Reddit.

## <a name="how-can-i-financially-support-freenet-development"></a>How can I financially support Freenet development?

Founded in 2001, Freenet is a 501c3 non-profit organization dedicated to
the development and propagation of technologies for open and democratic
information distribution over the Internet. We advocate for unrestricted
exchange of intellectual, scientific, literary, social, artistic,
creative, human rights, and cultural expressions, free from interference
by state, private, or special interests.

#### Donate via PayPal or Credit Card

[](https://www.paypal.com/donate?hosted_button_id=EQ9E7DPHB6ETY)

#### Donate via Cryptocurrency

Freenet is **not** a cryptocurrency, but we do accept cryptocurrency
donations. For large donations (over \$5,000) please contact us before
sending. For smaller donations, please use the following wallets:

  Cryptocurrency   Address
  ---------------- ---------
  Bitcoin          
  Zcash            
  Ethereum         
