+++
title = "Introducing Ghost Keys"
date = 2024-08-11
tags = [ "front-page"]
+++

### There Is No Negative Trust on the Internet

On May 3rd, 1978, Gary Thuerk, a marketing manager at Digital Equipment Corporation, sent the first
spam email to 400 people. It was an invitation to a product demonstration of the DEC-20 computer.
The response was immediate and negative.

Fast forward nearly 50 years, and the same flaw in the internet’s design has led to much bigger
problems. Today, we’re dealing with AI-driven bots that not only flood us with spam but also
manipulate social and political discourse on a massive scale.

This flaw has also given major platforms an excuse to limit interoperability through APIs—a
convenient way to lock users into their ecosystems and keep competitors out.

The result? Innovation has stalled, and manipulation and control are now the norm.

### Ghost Keys: Anonymous and Verifiable Identities

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

While Ghost Keys don’t come with ready-made applications, they offer a strong foundation for
developers to build on. We’ve also developed a command line tool that lets you use your Ghost Key
certificate and private key to sign messages and verify signatures. There’s a Rust library available
for the same functionality. These tools are just the beginning as we explore broader uses for Ghost
Keys in establishing trust and identity across the web.

Ghost Keys aim to rebuild trust online in a way that’s both decentralized and resistant to
manipulation while preserving user anonymity.
