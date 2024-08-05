Ghost keys address a crucial challenge on the Internet: establishing trust without sacrificing privacy. With personal 
data commoditized by big tech, ghost keys are a way to maintain anonymity while tackling serious problems like bots
and spam.

Here's how it works: when you make a donation to Freenet, your web browser generates a public-private key pair. The 
public key is then encrypted, or blinded, and sent to our server. The server signs this blinded key and sends it back 
to your browser, which decrypts the signature, creating a cryptographic certificate that is signed by the server without
the server ever seeing it. This process ensures that your identity is not linked to your donation, providing a unique 
certificate that proves you've invested value.

By linking trust to anonymity, ghost keys eliminate the need for cumbersome captchas. They block spam, prevent bots, 
and secure your interactions, making them a powerful tool for those who value privacy, security, and control over 
their digital presence.

### Technical Overview

1. On completion of a donation, the browser creates an [Ed25519](https://en.wikipedia.org/wiki/EdDSA) key pair.
2. The **public part** of the key pair is [blinded](https://www.rfc-editor.org/rfc/rfc9474.html) and sent to the server. 
3. The server verifies the donation and then signs the blinded key with it's RSA key. 
4. The blinded signature is sent back to the browser and unblinded.
5. The browser presents both the key pair and the unblinded signature to the user.

The JavaScript code for the browser side of this process can be reviewed [here](/js/donation-success.js).
l details, making it suitable for both technical and non-technical audiences interested in understanding how ghost keys work.