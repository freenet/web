When you make a donation to Freenet, your browser creates a [public-private key pair](https://en.wikipedia.org/wiki/Public-key_cryptography). 
The **public key** is encrypted, or "blinded," and sent to our server. This ensures that the server 
cannot associate the key pair with your donation, maintaining your anonymity. The server signs the blinded 
ublic key to certify your donation without ever seeing the actual key. Your browser then decrypts 
the signature, delivering a cryptographic certificate that signifies your invested value. This anonymously 
signed key, or "Ghost Key," can be used to verify sender authenticity in your communications, mitigate 
spam, and establish a credible identity within the decentralized reputation system—all while you 
remain anonymous.

### Technical Overview

1. On completion of a donation, the browser creates an [Ed25519](https://en.wikipedia.org/wiki/EdDSA) key pair.
2. The **public part** of the key pair is blinded and sent to the server. The server verifies the donation and then signs the blinded key.
3. The blinded signature is sent back to the browser and unblinded.
4. The browser presents both the key pair and the unblinded signature to the user.

The JavaScript code for the browser side of this process can be reviewed [here](/js/donation-success.js).
l details, making it suitable for both technical and non-technical audiences interested in understanding how ghost keys work.