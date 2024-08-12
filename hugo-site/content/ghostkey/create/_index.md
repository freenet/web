---
title: "Get Your Ghost Key"
date: 2024-07-10
draft: false
layout: "single"
---

After you donate to Freenet, you'll receive a [Ghost Key](/ghostkey/)—a cryptographic key certified
by Freenet. The key is generated in your browser, then "blinded" before being sent to our server.
The server signs the blinded key without ever seeing the unblinded version, ensuring that your
donation remains anonymous. Your browser then unblinds the signature, creating a signed certificate.

It's important to store this certificate securely, such as in a secure note within your password
manager. If you'd like to learn more about Ghost Keys before making a donation, learn more [here](/ghostkey/).

We use [Stripe](https://stripe.com/) for credit card processing.

{{< spacer >}}

{{< stripe-donation-form error-message="The Ghost Key back-end isn't currently running, please notify webmaster@freenet.org" >}}

{{< spacer >}}

<div id="certificateSection" style="display: none;">
  <h2>Your Ghost Key</h2>
  <p>Below is your Ghost Key. Please copy and save it securely.</p>
  <textarea id="combinedKey" rows="10" cols="72" readonly></textarea>
  <button id="copyCombinedKey">Copy Ghost Key</button>
</div>

<div id="errorMessage" style="display: none; color: red;"></div>

{{< include "ghost-key-explanation.md" >}}
