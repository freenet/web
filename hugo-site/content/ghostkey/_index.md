---
title: "Ghost Keys"
date: 2024-07-10
draft: false
layout: "single"
---

{{< bulma-button href="/ghostkey/create/" color="#339966" >}}Get Your Ghost Key{{< /bulma-button >}}
{{< bulma-button href="https://crates.io/crates/ghostkey" color="#3366cc" >}}Command Line Tool{{< /bulma-button >}}
{{< bulma-button href="https://crates.io/crates/ghostkey_lib" color="#cc6633" >}}Ghost Key Library{{< /bulma-button >}}

### Learn More

- [What is a Ghost Key?](#what-is-a-ghost-key)
- [How do Ghost Keys work?](#how-do-ghost-keys-work)
- [How much should I donate?](#how-much-should-i-donate)
- [What can I do with my Ghost Key?](#what-can-i-do-with-my-ghost-key)

# What is a Ghost Key? {#what-is-a-ghost-key}

Ghost keys address a crucial challenge on the Internet: establishing trust without sacrificing
privacy. With personal data commoditized by big tech, ghost keys are a way to maintain anonymity
while tackling serious problems like bots and spam.

Here's how it works: when you make a donation to Freenet, your web browser generates a
public-private key pair. The public key is then encrypted, or blinded, and sent to our server. The
server signs this blinded key and sends it back to your browser, which decrypts the signature,
creating a cryptographic certificate that is signed by the server without the server ever seeing it.
This process ensures that your identity is not linked to your donation, providing a unique
certificate that proves you've invested value.

By linking trust to anonymity, ghost keys eliminate the need for cumbersome captchas. They block
spam, prevent bots, and secure your interactions, making them a powerful tool for those who value
privacy, security, and control over their digital presence.

# How do Ghost Keys work? {#how-do-ghost-keys-work}

1. On completion of a donation, the browser creates an
   [elliptic curve](https://en.wikipedia.org/wiki/EdDSA) key pair.
2. The **public part** of the key pair is [blinded](https://www.rfc-editor.org/rfc/rfc9474.html) and
   sent to the server.
3. The server verifies the donation and then signs the blinded key with it's RSA key.
4. The blinded signature is sent back to the browser and unblinded.
5. The browser presents a certificate to the user along with the private key, proving the donation
   was made without revealing the user's identity.

# How much should I donate? {#how-much-should-i-donate}

The minimum donation is $5, but you can donate however much you can afford. The donation amount will
be securely recorded in your Ghost Key certificate, and in the future it's possible that access and
perks on the network will be reserved for those who donate more or who donated earlier. Think of it
like a "founding member" club.

# What can I do with my Ghost Key? {#what-can-i-do-with-my-ghost-key}

When you first receive your Ghost Key it's very important that you store it securely, perhaps as
a secure note in a password manager you trust.
