---
title: "Get Your Ghost Key"
date: 2024-07-10
draft: false
layout: "single"
---

After you donate to Freenet, you'll receive a certified [Ghost Key](/ghostkey/)—a cryptographic key.
The key is generated in your browser, then "blinded" before being sent to our server. The server
signs the blinded key without ever seeing the unblinded version, ensuring that your donation remains
anonymous. Your browser then unblinds the signature, creating a signed certificate.

It's important to store this certificate securely, such as in a secure note within your password
manager. You can read an article about Ghost Keys [here](/news/introducing-ghost-keys/).

We offer a $1 donation option to ensure that Ghost Keys are accessible to everyone, especially those
with limited means. This is the lowest amount we can offer without credit card processing fees
consuming most of the donation. However, if you're able to contribute more, we encourage you to do
so. Your generosity directly supports the ongoing development of Freenet, helping us build a more
private, secure, and decentralized internet. Your donation amount will be recorded in the Ghost Key
certificate and could provide additional benefits in the future.

We use [Stripe](https://stripe.com/) for credit card processing.

{{< spacer >}}

{{< stripe-donation-form error-message="The Ghost Key service is down, please notify webmaster@freenet.org" >}}
