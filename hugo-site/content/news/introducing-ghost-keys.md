+++
title = "Introducing Ghost Keys"
date = 2024-08-13
author = "Ian Clarke"
tags = [ "front-page"]
+++

<img src="/img/ghost-key-illustration.webp" alt="Ghost Key Illustration" style="float: right; width: 250px; height: 250px; margin-left: 20px;">

#### There Is No Negative Trust on the Internet

On May 3rd, 1978, Gary Thuerk, a marketing manager at Digital Equipment Corporation, sent the first
spam email to 400 people. It was an invitation to a product demonstration for the DEC-20 computer,
and the reaction was immediate and negative.

Nearly 50 years later, this same flaw in the internet's design has given rise to more significant
issues. Today, AI-driven bots not only overwhelm us with spam but also manipulate social and
political discourse at scale.

The root of the problem is that internet identities can be created at no cost. As a result, there's
no effective mechanism for "negative trust"—a bad reputation doesn't stick. This allows bad actors
to operate with near impunity, as they can easily generate new identities and continue their
activities.

#### Introducing Ghost Keys: Anonymous and Verifiable Identities

Ghost Keys offer a unique approach to addressing these issues by providing a way to certify
identities through a real-world action—a small donation to [Freenet](https://freenet.org/). This
allows users to establish trust without compromising privacy, offering a solution particularly
suited to decentralized systems.

When you donate to Freenet, your browser generates a public-private key pair. The public key is
[blinded](https://en.wikipedia.org/wiki/Blind_signature) and sent to our server for signing.
Importantly, this blinding ensures that the server never sees your actual public key, so it can't
connect it to your donation. Once your donation is confirmed, the server signs the blinded public
key and sends it back. Your browser then unblinds it, producing a signed public key that proves your
donation. This signed key, along with other data, forms a certificate that you can store securely.

This identity is backed by a real-world action—giving it "skin in the game." Unlike throwaway
accounts, Ghost Keys are designed to be persistent and valuable because they aren't free to create.
This makes them particularly well-suited for reputation systems where accountability matters.

#### Why Ghost Keys Are Important for Freenet

As a decentralized system, Freenet faces many of the same challenges that affect the broader
internet, such as spam and identity fraud. However, unlike centralized systems, Freenet can't rely
on a central authority to manage these issues. Ghost Keys offer a cryptographically verifiable
identity solution that aligns with Freenet's decentralized principles. Because Ghost Keys are tied
to a real-world action (a donation) and can be verified without any centralized service, they
provide a solid foundation for trust within the network.

Looking ahead, Ghost Keys are the first step in building a decentralized reputation system for
Freenet. This system will be built on the idea of a "web of trust," where trust can be extended
across the network. For example, if User A trusts User B, and User B trusts User C, then User A will
have a certain level of trust for User C, even without direct interaction.

As this system scales, it will create a decentralized trust network that allows users to assess the
credibility of others without relying on a centralized authority—an essential step for strengthening
Freenet's decentralized infrastructure.

#### Why Donations?

Since Freenet is decentralized, it may seem counterintuitive that we're using donations, which
involve a centralized process. However, there are clear reasons for this approach:

1. **Anonymity**: Although donations are processed centrally, the blind signature mechanism ensures
   that they remain anonymous.

2. **Support for a Larger Mission**: Donations are essential for funding Freenet's broader mission
   of decentralizing internet services. Ghost Keys are just one part of this effort.

3. **Simplicity**: Donations are straightforward and easy to understand, making them a practical
   starting point for establishing trust within the network.

While donations are the current method for supporting Freenet, we're actively exploring
decentralized alternatives. One example is
[Proof of Trust](/news/799-proof-of-trust-a-wealth-unbiased-consensus-mechanism-for-distributed-systems/),
a system that could help build decentralized trust without relying on Proof of Work or Proof of
Stake. These methods won't replace donations overnight but will expand the options available for
supporting Freenet in the future.

#### Command Line Tool and Rust Library

To further support developers, we've created a
[command line tool](https://crates.io/crates/ghostkey) that allows you to use your Ghost Key
certificate and private key to sign messages and verify signatures. Additionally, there is
[a Rust library](https://crates.io/crates/ghostkey_lib) for those who want to integrate Ghost Keys
into their own projects.

For example, installing and using the command line tool is straightforward:

```bash
# Install the Ghost Key command line tool (you'll need cargo and Rust)
$ cargo install ghostkey
$ ghostkey verify-ghost-key --ghost-certificate ~/Downloads/ghost-key-cert.pem
Ghost certificate verified
Info: {"action":"freenet-donation","amount":20,"delegate-key-created":"2024-07-30 15:39:26"}

# Sign a message
$ ghostkey sign-message --ghost-certificate ~/Downloads/ghost-key-cert.pem \
          --ghost-signing-key ~/Downloads/ghost-key-signing-key.pem \
          --message "Hello, World!" --output signed_message.pem

# Verify a signed message
$ ghostkey verify-signed-message --signed-message signed_message.pem
Ghost certificate verified
Info: {"action":"freenet-donation","amount":20,"delegate-key-created":"2024-07-30 15:39:26"}
Signature verified
Message: Hello, World!
```

This tool demonstrates how to sign messages with your Ghost Key certificate and signing key, and how
to verify those signed messages. The verification process ensures the integrity of both the Ghost
Key certificate and the message signature.

Ghost Keys represent a meaningful advancement in addressing the challenge of trust in decentralized
systems. By linking identities to real-world actions while preserving privacy, Ghost Keys provide a
scalable, decentralized solution to the absence of negative trust on the internet. They form the
foundation of a more resilient reputation system that aligns with Freenet's decentralized principles
and vision for the future.

#### What's next?

- [Frequently Asked Questions](/ghostkey/) - Includes more technical details
- [Command Line Tool](https://crates.io/crates/ghostkey) - Quick to install using cargo
- [Rust Library](https://crates.io/crates/ghostkey_lib) - For developers who want to integrate Ghost
  Keys into their projects

{{< bulma-button href="/ghostkey/create/" color="#339966" >}}Donate to Get Your Ghost
Key{{< /bulma-button >}}

<p></p>

🙏 *With gratitude to Steven Starr for his help with this article.*
