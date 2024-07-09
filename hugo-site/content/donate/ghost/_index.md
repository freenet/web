---
title: "Donate to Freenet and Receive a Ghost Credential"
date: 2024-06-24
draft: false
cascade:
    _build:
        list: false
        render: false
---

{{< stripe-donation-form error-message="The Ghost Credential back-end isn't currently running, please notify webmaster@freenet.org" >}}

When you make a donation to Freenet, your browser creates a cryptographic key that is then encrypted and sent to our 
server. We sign this encrypted key to certify your donation without ever seeing your actual key. This ensures that 
your anonymity is maintained and prevents any connection between your donation and the key. The signed key, now 
called a "Ghost Credential," can be used to verify sender authenticity in your communications, mitigate spam, and 
establish a credible identity within the decentralized reputation system—all while keeping your privacy intact.
