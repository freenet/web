---
title: "Freenet Ghost Key"
date: 2024-07-10
draft: false
layout: "single"
---

{{< bulma-button href="/ghostkey/create/" color="#339966" >}}Donate to Get Your Ghost
Key{{< /bulma-button >}}

### Learn More

- [What is a Ghost Key?](#what-is-a-ghost-key)
- [How do Ghost Keys work?](#how-do-ghost-keys-work)
- [How much should I donate?](#how-much-should-i-donate)
- [How do I store my Ghost Key?](#how-do-i-store-my-ghost-key)
- [How do I use my Ghost Key?](#how-do-i-use-my-ghost-key)

You can also read an [introductory article about Ghost Keys](/news/introducing-ghost-keys/), or
[watch a discussion](/news/ghost-keys-ian-interview/) about it.

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

# How Do Ghost Keys Work? {#how-do-ghost-keys-work}

1. After a donation is completed the browser generates an
   [elliptic curve](https://en.wikipedia.org/wiki/EdDSA) key pair.
2. The **public key** is [blinded](https://www.rfc-editor.org/rfc/rfc9474.html) and sent to the
   server.
3. The server verifies the donation and signs the blinded public key with its RSA key.
4. The server then sends the blinded signature back to the browser, which unblinds it.
5. The browser combines the unblinded signature with a certificate that authenticates the server's
   signing key (known as the delegate key), the donation amount, and the date the delegate key was
   created.
6. Finally, the browser presents this certificate to the user along with a corresponding signing
   key, proving the donation was made without revealing the user's identity.
7. The user stores this certificate and signing key safely

# How much should I donate? {#how-much-should-i-donate}

The minimum donation is just $1, but you can donate however much you can afford. The donation amount
will be securely recorded in your Ghost Key certificate, and in the future it's possible that access
and perks on the network will be reserved for those who donate more or who donated earlier. Think of
it like a "founding member" club.

# How do I store my Ghost Key? {#how-do-i-store-my-ghost-key}

If you have a Freenet peer running on your computer, click "Import to Freenet" on the success page
after purchasing. This imports your Ghost Key into a
[delegate](https://freenet.org/resources/manual/components/delegates/) on your node -- a secure
local agent that stores your signing key and handles identity operations on your behalf. Your
signing key never leaves your node.

We also recommend downloading your Ghost Key as a backup in case you need to set up a new node.

# How do I use my Ghost Key? {#how-do-i-use-my-ghost-key}

Once imported, applications can request to use your Ghost Key for identity verification through the
delegate. You'll be prompted for permission each time.

You can also use our [command line tool](https://crates.io/crates/ghostkey) to verify your Ghost
Key and use it to sign and verify messages. Here is how to install and use it:

```
# If necessary install Rust
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install the Ghost Key command line tool
$ cargo install ghostkey

# Verify it's working
$ ghostkey -h
Utility for generating and verifying Freenet ghost keys. Use 'ghostkey <subcommand> -h'
for help on specific subcommands.

Usage: ghostkey [COMMAND]

Commands:
  verify-ghost-key       Verifies a ghost certificate
  generate-master-key    Generate a new master keypair
  generate-delegate      Generates a new delegate signing key and certificate
  verify-delegate        Verifies a delegate key certificate using the master verifying key
  generate-ghost-key     Generates a ghost key from a delegate signing key
  sign-message           Signs a message using a ghost key
  verify-signed-message  Verifies a signed message
  help                   Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version

$ ghostkey verify-ghost-key --ghost-certificate ~/Downloads/ghost-key-cert.pem
Ghost certificate verified
Info: {"action":"freenet-donation","amount":20,"delegate-key-created":"2024-07-30 15:39:26"}
```
