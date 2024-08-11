+++
title = "Introducing Freenet's Ghost Keys"
date = 2024-08-11
tags = [ "front-page"]
+++

### There Is No Negative Trust on the Internet

#### The Flaw in the Internet’s Design

On May 3rd, 1978, Gary Thuerk, a marketing manager at Digital Equipment Corporation, sent the first
spam email to 400 people. It was an invitation to a product demonstration of the DEC-20 computer.
The response was immediate and negative.

Fast forward nearly 50 years, and the same flaw in the internet’s design has led to much bigger
problems. Today, we’re dealing with AI-driven bots that not only flood us with spam but also
manipulate social and political discourse on a massive scale.

This flaw is symptomatic of a deeper issue: the internet was built without a system for establishing
negative trust. While positive interactions can be amplified, there's no effective mechanism to
recognize and counteract malicious behavior in real-time. This lack of negative trust has allowed
manipulation, misinformation, and centralized control to thrive unchecked.

#### Introducing Ghost Keys: Anonymous and Verifiable Identities

Ghost Keys offer a way to address these issues by enabling anonymous yet verifiable identities
online. They help solve the problem of trust in decentralized environments without compromising user
privacy.

When you donate to Freenet, your browser generates a public-private key pair. The public key is
blinded and sent to our server for signing. Once your donation is confirmed, the server signs the
blinded public key and sends it back. Your browser then unblinds the key, creating a signed public
key that proves your donation. This signed key, along with other data, forms a certificate you can
store securely.

This identity is tied to a real action—it has a cost, which makes it less likely to be abused.
Unlike throwaway accounts, Ghost Keys are designed to be persistent and valuable because they aren't
free to create. This makes them ideal for reputation systems, where having "skin in the game"
matters.

#### Why Donations Are Appropriate

Given that Freenet is a decentralized project, one might question why we rely on donations, which
are inherently centralized. The answer lies in three key factors:

1. **Anonymity**: Although donations involve a central process, they are designed to be anonymous,
   which aligns with the privacy principles at the core of Freenet. The anonymity mitigates the
   centralization aspect, ensuring that user privacy remains protected.

2. **Funding for a Larger Mission**: Donations provide essential funding to Freenet, which not only
   developed Ghost Keys but is also tackling the much broader challenge of decentralizing virtually
   all internet services. Without this funding, progress on these ambitious goals would be severely
   hampered.

3. **Simplicity**: Donations are a straightforward and easily understood method for supporting
   Freenet. They allow people to contribute without needing to understand more complex or
   experimental funding mechanisms.

While donations are our current method of support, we’re actively exploring decentralized
alternatives for the future. For example, we plan to implement a system called Proof of Trust, which
offers a decentralized way to build trust without relying on Proof of Work or Proof of Stake. (For
more details, you can visit [URL]). This system is based on the difficulty of finding others with
whom you can play a game that requires mutual trust. The point is that donations offer a firm basis
on which to build, but they won’t be the only option as we continue to innovate.

#### Looking Forward: The Future of Ghost Keys and Decentralized Trust

While Ghost Keys don’t come with ready-made applications, they offer a strong foundation for
developers to build on. We’ve also developed a command line tool that lets you use your Ghost Key
certificate and private key to sign messages and verify signatures. There’s a Rust library available
for the same functionality. These tools are just the beginning as we explore broader uses for Ghost
Keys in establishing trust and identity across the web.

Ghost Keys aim to rebuild trust online in a way that’s both decentralized and resistant to
manipulation while preserving user anonymity. By starting with donations and moving towards
decentralized possibilities, we’re laying the groundwork for a future where trust is securely
established, and the absence of negative trust is no longer a vulnerability but a strength.
 #### 