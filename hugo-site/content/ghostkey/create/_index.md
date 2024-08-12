---
title: "Get Your Ghost Key"
date: 2024-07-10
draft: false
layout: "single"
---

Use your credit card to donate to Freenet, after you make your donation you can copy your ghost key
and certificate from your browser. The ghost key is a cryptographic key that proves you've donated to Freenet,
you must keep it secret (particularly the key part).

We use [Stripe](https://stripe.com/) for credit card processing.

{{< spacer >}}

{{< stripe-donation-form error-message="The Ghost Key back-end isn't currently running, please notify webmaster@freenet.org" >}}

{{< spacer >}}

{{< bulma-button href="/ghostkey/" color="#339966" >}}Learn More About Ghost Keys{{< /bulma-button >}}

<div id="certificateSection" style="display: none;">
  <h2>Your Ghost Key</h2>
  <p>Below is your Ghost Key. Please copy and save it securely.</p>
  <textarea id="combinedKey" rows="10" cols="72" readonly></textarea>
  <button id="copyCombinedKey">Copy Ghost Key</button>
</div>

<div id="errorMessage" style="display: none; color: red;"></div>

{{< include "ghost-key-explanation.md" >}}
