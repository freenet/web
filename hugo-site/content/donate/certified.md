---
title: "Certified Donations"
date: 2024-06-24
draft: true
---

### Certified Donations

Thanks for your interest in supporting Freenet.

When you make a donation to Freenet, we want to give you a special certificate
to acknowledge your contribution. This certificate isn't just a simple thank
you—it's a digital token that could unlock additional features within Freenet in
the future. Here's how it works, broken down step-by-step:

1. **Key Generation**: When you decide to make a donation, your web browser will
   generate a unique pair of cryptographic keys for you. Think of these as a
   lock (public key) and key (private key). 

2. **Blinding for Privacy**: To ensure your privacy and security, your public
   key (the lock) is "blinded." This means it's scrambled in a way that hides
   its true form. This blinded key is then sent to our server.

3. **Donation and Signing**: When you complete your donation, our server will
   use a special digital signature to sign your blinded public key. Different
   donation amounts are signed with different keys to recognize varying levels
   of support.

4. **Unblinding**: After signing, the server sends the signed, still-blinded
   public key back to your browser. Your browser then "unblinds" it, revealing
   the signed version of your original public key.

5. **Your Certificate**: Finally, your browser gives you:
   - The private key (the key to your lock), encoded in a way you can easily
     save.
   - The signed public key (the lock), which proves your donation and its
     amount.

### Why This Matters

This cryptographic certificate is unique to you and your donation. It proves
that you supported Freenet at a specific level without revealing any personal
information. While these certificates are not transferable (sharing your private
key would defeat the purpose), they can serve as a foundation for future
features, like a reputation system within Freenet. This could help reduce spam
by ensuring only genuine supporters can send certain types of messages or access
specific features.

### Clear and Secure

Unlike NFTs or other digital tokens often associated with speculative trading,
our donation certificates are designed purely for functionality within the
Freenet ecosystem. They are not meant to be traded or sold but to enhance your
experience and recognition within the Freenet community.

By donating, you're not only supporting the development of Freenet but also
receiving a secure, private token of appreciation that could offer additional
benefits down the line.
