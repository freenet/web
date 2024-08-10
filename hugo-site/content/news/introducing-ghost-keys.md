**Title:** Introducing Ghost Keys  
**Date:** 2024-08-11  
**Tags:** [ "front-page" ]

### There is No Negative Trust on the Internet

On May 3rd, 1978, Gary Thuerk, a marketing manager at Digital Equipment Corporation, sent the first spam email to 400
recipients. The message, an invitation to a product demonstration of the DEC-20 computer, was met with swift and
negative reactions.

Nearly half a century later, this fundamental flaw in internet design—where there is no mechanism for negative trust—has
evolved into a significant problem. Sophisticated AI-driven spam and bots now manipulate us through social media,
exploiting the absence of a system that could enforce accountability or distinguish between trustworthy and
untrustworthy entities.

To counter this, online services rely on captchas—a cumbersome, often frustrating solution that poses accessibility
challenges and does little to stop advanced bots. More insidiously, the lack of negative trust has allowed monopolistic
online services to limit interoperability through restrictive APIs, locking in users and stifling competition.

### Enter Ghost Keys: Trust Without Sacrificing Privacy

Ghost Keys offer a revolutionary solution by providing a way to establish trust online without compromising privacy. In
a digital landscape where personal data is often commodified, Ghost Keys enable you to prove your identity and intent
securely and anonymously, without exposing personal details.

Here’s how Ghost Keys work:

1. **Key Generation:** When you make a donation to Freenet, your web browser generates an elliptic curve key pair.
2. **Blinding the Public Key:** The public part of the key pair is blinded, meaning it is encrypted in a way that
   obscures its content from the server.
3. **Server-Side Signing:** The blinded key is sent to the server, which verifies the donation and signs the blinded key
   with its RSA key.
4. **Unblinding and Certificate Creation:** The blinded signature is returned to your browser, which unblinds it to
   create a cryptographic certificate. This certificate, along with the private key, serves as proof of your donation
   without linking your identity to it.

By linking trust to anonymity, Ghost Keys eliminate the need for captchas, block spam, and prevent bots—all while
preserving your privacy. Whether you're using Freenet or engaging in other online activities, Ghost Keys provide a
powerful tool for those who value security and control over their digital presence.

---

This version retains the reference to "no negative trust on the internet" and emphasizes its importance in understanding
the broader issue. It should resonate well with both those familiar with the concept and those curious to learn more.
