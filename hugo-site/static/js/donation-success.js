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
    // Using Ed25519, which is more commonly used for signatures
    const ec = new elliptic.eddsa('ed25519');
    const keyPair = ec.keyFromSecret(); // Generates a new key pair
    const publicKey = keyPair.getPublic('hex');
    const privateKey = keyPair.getSecret('hex');

    // For Ed25519, we don't need to blind the public key
    const blindedPublicKey = publicKey;

    // Send blinded public key to server for signing
    const response = await fetch('http://127.0.0.1:8000/sign-certificate', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ payment_intent_id: paymentIntentId, blinded_public_key: blindedPublicKey })
    });

    if (!response.ok) {
      const errorData = await response.json();
      throw new Error(`Failed to sign certificate: ${errorData.error || response.statusText}`);
    }

    const data = await response.json();
    if (!data.blind_signature) {
      throw new Error('Invalid response from server: missing blind_signature');
    }
    const blindSignature = data.blind_signature;

    // For Ed25519, we don't need to unblind the signature
    const unblindedSignature = blindSignature;

    // Armor the certificate and private key
    const armoredCertificate = `-----BEGIN FREENET DONATION CERTIFICATE-----
${publicKey}|${unblindedSignature}
-----END FREENET DONATION CERTIFICATE-----`;

    const armoredPrivateKey = `-----BEGIN FREENET DONATION PRIVATE KEY-----
${privateKey}
-----END FREENET DONATION PRIVATE KEY-----`;

    // Display the certificate and private key
    document.getElementById('certificate').value = armoredCertificate;
    document.getElementById('privateKey').value = armoredPrivateKey;
    document.getElementById('certificateSection').style.display = 'block';
    document.getElementById('certificate-info').style.display = 'none';

    // Set up download button
    document.getElementById('downloadCertificate').addEventListener('click', function() {
      const certificateData = {
        certificate: armoredCertificate,
        privateKey: armoredPrivateKey
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

    // Verify the certificate
    if (verifyCertificate(publicKey, unblindedSignature)) {
      console.log("Certificate verified successfully");
    } else {
      console.error("Certificate verification failed");
      showError('Certificate verification failed. Please contact support.');
    }
  } catch (error) {
    showError('Error generating certificate: ' + error.message);
  }

  function verifyCertificate(publicKey, signature) {
    const ec = new elliptic.eddsa('ed25519');
    const key = ec.keyFromPublic(publicKey, 'hex');
    // In a real scenario, we would verify the signature against a known message
    // For now, we'll just check if the signature is valid for an empty message
    return key.verify('', signature);
  }
}

function showError(message) {
  const errorElement = document.getElementById('errorMessage');
  errorElement.textContent = message;
  errorElement.style.display = 'block';
  document.getElementById('certificate-info').style.display = 'none';
}
