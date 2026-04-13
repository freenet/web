---
title: "Get Your Ghost Key"
date: 2024-07-10
draft: false
layout: "single"
---

After you donate to Freenet, you'll receive a certified [Ghost Key](/ghostkey/), a cryptographic key.
The key is generated in your browser, then "blinded" before being sent to our server. The server
signs the blinded key without ever seeing the unblinded version, ensuring that your donation remains
anonymous. Your browser then unblinds the signature, creating a signed certificate.

> **⚠️ Back up your Ghost Key before importing it to Freenet.** Save both the
> certificate and the signing key to a password manager **before** clicking
> **Import to Freenet** on the success page. The Ghostkey Vault delegate is still
> early software and keys have been observed disappearing from the vault; without a
> backup, a lost key and the donation behind it cannot be recovered. Tracked in
> [freenet/ghostkeys#3](https://github.com/freenet/ghostkeys/issues/3).

You can read an article about Ghost Keys [here](/about/news/introducing-ghost-keys/).

We offer a $1 donation option to ensure that Ghost Keys are accessible to everyone, especially those
with limited means. Your generosity directly supports the ongoing development of Freenet, helping us
build a more private, secure, and decentralized internet. Your donation amount will be recorded in
the Ghost Key certificate and could provide additional benefits in the future.

We use [Stripe](https://stripe.com/) for credit card processing.

{{< spacer >}}

{{< stripe-donation-form error-message="The Ghost Key service is down, please notify webmaster@freenet.org" >}}
