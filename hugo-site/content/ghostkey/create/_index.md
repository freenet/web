---
title: "Get Your Ghost Key"
date: 2024-07-10
draft: false
layout: "single"
---

Your browser generates a key and *blinds* it before sending it to our server. The server confirms
your donation and signs a key it cannot read. Your browser then unblinds that signature, producing
your [Ghost Key](/ghostkey/) certificate. The result is an identity we can verify but cannot connect
to your payment.

> **Before you pay:** after donating you'll download your certificate and signing key, and that
> download is the authoritative copy. Keep it somewhere safe, such as a secure note in a password
> manager. The Ghostkey Vault you can import it into is still experimental and can currently lose
> keys ([freenet/ghostkeys#3](https://github.com/freenet/ghostkeys/issues/3)), so the backup is what
> protects you.

The minimum is **$1**. We use [Stripe](https://stripe.com/) for card processing. Freenet Project Inc
is a 501(c)(3) nonprofit, and contributions are tax-deductible in the United States.

{{< spacer >}}

{{< stripe-donation-form error-message="The Ghost Key service is down, please notify webmaster@freenet.org" >}}

{{< spacer >}}

## Who sees what

- **Stripe**, our payment processor: your card details and the amount. Stripe never sees your Ghost
  Key, blinded or otherwise.
- **Freenet's donation server**: your payment record from Stripe, which it has to read in order to
  confirm the payment succeeded, and separately a blinded key that it is mathematically unable to
  read.
- **Anyone verifying your key later**: that a valid Ghost Key was issued at a given donation tier.
  Not your name, not your payment.

The unblinding happens in your browser, on a key our server never sees in unblinded form. So while
our server does handle your payment, the link between that payment and the Ghost Key you end up
holding is never created anywhere, including on our own machines.

## Why the amounts are fixed

You choose from a short list rather than typing your own figure. The donation amount is recorded in
your certificate and is visible to anyone who verifies it, so fixed tiers keep your certificate
indistinguishable from every other one issued at the same amount. A free-form figure like $37.42
would be close to unique, and would give away much of what blind signing is there to protect.

The $1 tier exists so that Ghost Keys stay within reach of people with limited means. Anything above
it directly funds Freenet's development, and apps that want to extend extra privileges to larger
donors can read the tier from your certificate.
