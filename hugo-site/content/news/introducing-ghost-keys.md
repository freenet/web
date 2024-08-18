+++
title = "Introducing Ghost Keys"
date = 2024-08-13
tags = [ "front-page"]
+++

<img src="/img/ghost-key-illustration.webp" alt="Ghost Key Illustration" style="float: right; width: 250px; height: 250px; margin-left: 20px;">

#### There Is No Negative Trust on the Internet

On May 3rd, 1978, Gary Thuerk, a marketing manager at Digital Equipment Corporation, sent the first
spam email to 400 people. It was an invitation to a product demonstration for the DEC-20 computer,
and the reaction was immediate and negative.

Nearly 50 years later, the same underlying flaw in the internet's design has led to far greater
problems. Today, AI-driven bots not only inundate us with spam but also manipulate social and
political discourse on a massive scale.

This flaw highlights a deeper issue: when new internet identities can be created without any cost,
there is no mechanism for negative trust. Bad reputations don't stick. This allows bad actors to
exploit the system with almost no consequences.

#### Introducing Ghost Keys: Anonymous and Verifiable Identities

Ghost Keys offer a novel solution to these challenges by allowing users to certify a new identity
through a real-world action, a small donation to [Freenet](https://freenet.org/). This approach lets
users establish trust online without compromising their privacy, and effectively addresses the trust
issue in decentralized environments while maintaining user anonymity.

When you donate to Freenet, your browser generates a public-private key pair. The public key is
blinded and sent to our server for signing. Crucially, _the blinding mechanism means the server
never sees your actual public key and thus can never connect it to your donation_. Once your
donation is confirmed, the server signs the blinded public key and sends it back. Your browser then
unblinds the key, creating a signed public key that proves your donation. This signed key, along
with other data, forms a certificate you can then store securely.

This identity is tied to a real action—it has a cost, which makes it less likely to be abused.
Unlike throwaway accounts, Ghost Keys are designed to be persistent and valuable because they aren't
free to create. This makes them ideal for reputation systems, where having "skin in the game"
matters.

#### Why Ghost Keys Are Essential for Freenet

As a decentralized system, Freenet must address the same vulnerabilities that plague the broader
internet, such as spam and identity fraud, without relying on centralized authorities. Ghost Keys
provide a cryptographically verifiable identity solution that is perfectly aligned with Freenet's
decentralized ethos. Since Ghost Keys are created through a real-world action (a donation) and can
be verified without any centralized service, they offer a robust foundation for establishing trust
within the Freenet network.

Looking ahead, Ghost Keys will serve as the cornerstone of a powerful decentralized reputation
mechanism. This system will build on the concept of a "web of trust," where trust can be extended
transitively across the network. For example, if User A trusts User B, and User B trusts User C,
then User A will inherit a degree of trust for User C.

At scale, this will create a trust network that empowers users to assess the trustworthiness of
others they haven't directly interacted with, strengthening the decentralized nature of Freenet.

#### Why Donations?

Given that Freenet is a decentralized project, one might question why we rely on donations, which
are inherently centralized. There are three key reasons:

1. **Anonymity**: Although the donation itself relies on a centralized system, they are anonymous
   thanks to the blind signature process.

2. **Funding for a Larger Mission**: Donations provide essential funding to Freenet, which not only
   developed Ghost Keys but is also tackling the much broader challenge of decentralizing virtually
   all internet services. It's a great cause.

3. **Simplicity**: Donations are straightforward and easily understood.

While donations are Freenet's current method of support, we’re actively exploring decentralized
alternatives for the future. For example, we plan to implement a system called
[Proof of Trust](/news/799-proof-of-trust-a-wealth-unbiased-consensus-mechanism-for-distributed-systems/),
which offers a decentralized way to build trust without relying on Proof of Work or Proof of Stake.
This system is based on the difficulty of finding others with whom you can play a game that requires
mutual trust. The point is that donations offer a firm basis on which to build, but they won’t be
the only option as we continue to innovate.

#### Command Line Tool and Rust Library

Ghost Keys offer a strong foundation for developers to build on. We’ve also developed a
[command line tool](https://crates.io/crates/ghostkey) that lets you use your Ghost Key certificate
and private key to sign messages and verify signatures. There’s also
[a Rust library](https://crates.io/crates/ghostkey_lib).

Installing and using the command line tool is simple:

```
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

This demonstrates how to sign a message using your Ghost Key certificate and signing key, and then verify the signed message. The verification process checks both the Ghost Key certificate and the signature on the message.

Ghost Keys provide a significant advancement in addressing the challenge of trust in decentralized
systems. By anchoring identities to real-world actions while preserving privacy, Ghost Keys offer a
scalable solution to the issue of negative trust on the internet. Aligned with Freenet's
decentralized principles, they form a foundation for more resilient reputation systems free from
reliance on centralized authorities.

{{< bulma-button href="/ghostkey/create/" color="#339966" >}}Donate to Get Your Ghost
Key{{< /bulma-button >}}
