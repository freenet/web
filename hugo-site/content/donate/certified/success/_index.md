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
  <p>Public Key: <span id="publicKey"></span></p>
  <p>Signature: <span id="signature"></span></p>
  <button id="downloadCertificate">Download Certificate</button>
</div>

<div id="errorMessage" style="display: none; color: red;"></div>

<script src="https://cdnjs.cloudflare.com/ajax/libs/jsencrypt/3.2.1/jsencrypt.min.js"></script>
<script>
document.addEventListener('DOMContentLoaded', function() {
  const urlParams = new URLSearchParams(window.location.search);
  const paymentIntent = urlParams.get('payment_intent');
  const clientSecret = urlParams.get('payment_intent_client_secret');

  if (paymentIntent && clientSecret) {
    verifyPaymentAndGenerateCertificate(paymentIntent);
  } else {
    showError('Payment information not found.');
  }
});

async function verifyPaymentAndGenerateCertificate(paymentIntentId) {
  try {
    const response = await fetch(`http://127.0.0.1:8000/verify-payment/${paymentIntentId}`);
    if (response.ok) {
      generateCertificate(paymentIntentId);
    } else {
      showError('Payment verification failed. Please contact support.');
    }
  } catch (error) {
    showError('Error verifying payment: ' + error.message);
  }
}

function generateCertificate(paymentIntentId) {
  const crypt = new JSEncrypt({default_key_size: 2048});
  const privateKey = crypt.getPrivateKey();
  const publicKey = crypt.getPublicKey();

  // In a real-world scenario, you would send the public key to the server for signing
  // For this example, we'll just use a placeholder signature
  const signature = 'PLACEHOLDER_SIGNATURE';

  document.getElementById('publicKey').textContent = publicKey;
  document.getElementById('signature').textContent = signature;
  document.getElementById('certificateSection').style.display = 'block';
  document.getElementById('certificate-info').style.display = 'none';

  document.getElementById('downloadCertificate').addEventListener('click', function() {
    const certificateData = {
      paymentIntentId: paymentIntentId,
      publicKey: publicKey,
      signature: signature,
      privateKey: privateKey // Note: In a real-world scenario, you wouldn't include the private key in the download
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
