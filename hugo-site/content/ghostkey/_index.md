---
title: "Donate to Freenet and Get a Ghost Key"
date: 2024-07-10
draft: false
url: "/ghostkey/"
layout: "single"
---

{{< stripe-donation-form error-message="The Ghost Key back-end isn't currently running, please notify webmaster@freenet.org" >}}

<p>Note: Our donation process has been updated. You'll be redirected to a secure Stripe page to complete your donation.</p>

{{< spacer >}}

<div id="certificateSection" style="display: none;">
  <h2>Your Ghost Key</h2>
  <p>Below is your Ghost Key. Please copy and save it securely.</p>
  <textarea id="combinedKey" rows="10" cols="72" readonly></textarea>
  <button id="copyCombinedKey">Copy Ghost Key</button>
</div>

<div id="errorMessage" style="display: none; color: red;"></div>

{{< include "ghost-key-explanation.md" >}}
