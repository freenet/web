---
title: "Donation Successful"
date: 2024-06-24
draft: false
url: "/donate/certified/success"
---

## Thank You for Your Donation!

Your donation to Freenet has been successfully processed. We greatly appreciate your support!

<div id="certificate-info">
  <p>Your donation certificate is being generated. Please wait...</p>
</div>

<div id="certificateSection" style="display: none;">
  <h3>Your Donation Certificate</h3>
  <p>Signed Public Key (base64):</p>
  <textarea id="signedPublicKey" rows="4" cols="50" readonly></textarea>
  <p>Private Key (base64):</p>
  <textarea id="privateKey" rows="4" cols="50" readonly></textarea>
  <button id="downloadCertificate">Download Certificate</button>
</div>

<div id="errorMessage" style="display: none; color: red;"></div>

<script src="https://cdn.jsdelivr.net/npm/elliptic@6.5.4/dist/elliptic.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/js-sha256@0.9.0/src/sha256.min.js"></script>
<script>
document.addEventListener('DOMContentLoaded', function() {
  const urlParams = new URLSearchParams(window.location.search);
  const paymentIntent = urlParams.get('payment_intent');

  if (paymentIntent) {
    generateAndSignCertificate(paymentIntent);
  } else {
    showError('Payment information not found.');
  }
});

async function generateAndSignCertificate(paymentIntentId) {
  try {
    // Generate EC key pair
    const ec = new elliptic.ec('secp256k1');
    const keyPair = ec.genKeyPair();
    const publicKey = keyPair.getPublic('hex');
    const privateKey = keyPair.getPrivate('hex');

    // Blind the public key
    const blindingFactor = ec.genKeyPair().getPrivate('hex');
    const blindedPublicKey = ec.g.mul(blindingFactor).encode('hex');

    // Send blinded public key to server for signing
    const response = await fetch('http://127.0.0.1:8000/sign-certificate', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ paymentIntentId, blindedPublicKey })
    });

    if (!response.ok) {
      throw new Error('Failed to sign certificate');
    }

    const { blindSignature } = await response.json();

    // Unblind the signature
    const unblindedSignature = ec.g.mul(ec.keyFromPrivate(blindSignature, 'hex').getPrivate())
      .add(ec.g.mul(blindingFactor).neg())
      .encode('hex');

    // Display the certificate
    document.getElementById('signedPublicKey').value = btoa(publicKey + '|' + unblindedSignature);
    document.getElementById('privateKey').value = btoa(privateKey);
    document.getElementById('certificateSection').style.display = 'block';
    document.getElementById('certificate-info').style.display = 'none';

    // Set up download button
    document.getElementById('downloadCertificate').addEventListener('click', function() {
      const certificateData = {
        publicKey: publicKey,
        signature: unblindedSignature,
        privateKey: privateKey
      };
      const blob = new Blob([JSON.stringify(certificateData, null, 2)], {type: 'application/json'});
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = 'freenet_donation_certificate.json';
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    });
  } catch (error) {
    showError('Error generating certificate: ' + error.message);
  }
}

function showError(message) {
  const errorElement = document.getElementById('errorMessage');
  errorElement.textContent = message;
  errorElement.style.display = 'block';
  document.getElementById('certificate-info').style.display = 'none';
}
</script>

If you have any questions or concerns, please don't hesitate to [contact us](/community/support).

Thank you again for supporting Freenet!
