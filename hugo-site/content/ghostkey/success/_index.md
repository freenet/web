---
title: "Ghost Key Donation Success"
date: 2023-07-10
draft: false
---

Thank you for your Ghost Key donation to Freenet! Your support helps us maintain and improve the
network while preserving your privacy.

Your Ghost Key certificate is being generated. This unique cryptographic token proves your donation
without revealing your identity. Please wait a moment while we process your information.

{{< donation-success >}}

## What's Next?

> **⚠️ Back up first, import second.** The Ghostkey Vault delegate is still early
> software and keys have been observed disappearing from the vault after import.
> **Save your Ghost Key before clicking Import to Freenet.** If it's lost from the
> vault and you have no backup, the key and the donation behind it cannot be recovered.
> Tracked in [freenet/ghostkeys#3](https://github.com/freenet/ghostkeys/issues/3).

**1. Backup (do this first):** Download your Ghost Key and save both the certificate
and the signing key somewhere safe, such as a secure note in a password manager. This
is currently the authoritative copy of your key. Treat the vault as a convenience, not
as storage you can rely on.

**2. Import to Freenet:** Once you've backed up, if you have a Freenet peer running on
this computer, click "Import to Freenet" to add your Ghost Key to your local identity
vault. Your Freenet peer must be running before you click this button. If you lose
access to your Freenet peer (or the vault loses the key), you can re-import from the
backup.

{{< bulma-button href="/ghostkey/" color="#339966" >}}Ghost Key FAQ{{< /bulma-button >}}
