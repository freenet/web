---
title: "Donate to Freenet and Receive a Ghost Key"
date: 2024-06-24
draft: false
layout: single
---

{{< stripe-donation-form error-message="The Ghost Key back-end isn't currently running, please notify webmaster@freenet.org" >}}

When you make a donation to Freenet, your browser creates a cryptographic 
secret key that is then encrypted and sent to our 
server. We sign this encrypted key to certify your donation without ever 
seeing your actual key. Your browser then decrypts the signature. This ensures 
that 
your anonymity is maintained and prevents any connection between your donation and the key. This anonymously signed 
key or "Ghost Key," can be used to verify sender authenticity in your communications, mitigate spam, and 
establish a credible identity within the decentralized reputation system—all while you remain anonymous.

### Technical Overview

1. On completion of a donation the browser creates an [Ed25519](https://en.
   wikipedia.org/wiki/EdDSA) key pair. 
2. The public part of the key pair is blinded and sent to the server. The 
   server verifies the donation and then signs the blinded key.
3. The blinded signature is sent back to the browser and unblinded.
4. The browser presents both the key pair and the unblinded signature to the 
   user.

The JavaScript code for the browser side of this process can be reviewed
[here](/js/donation-success.js).