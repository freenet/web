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
    // Using Curve25519, see: http://safecurves.cr.yp.to/
    const ec = new elliptic.ec('curve25519');
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

    // Unblind the signature
    const blindSignaturePoint = ec.keyFromPublic(blindSignature, 'hex').getPublic();
    const unblindedSignature = blindSignaturePoint.add(ec.g.mul(blindingFactor).neg()).encode('hex');

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
    const ec = new elliptic.ec('curve25519');
    const publicKeyPoint = ec.keyFromPublic(publicKey, 'hex').getPublic();
    const signaturePoint = ec.keyFromPublic(signature, 'hex').getPublic();
    return publicKeyPoint.eq(signaturePoint);
  }
}

function showError(message) {
  const errorElement = document.getElementById('errorMessage');
  errorElement.textContent = message;
  errorElement.style.display = 'block';
  document.getElementById('certificate-info').style.display = 'none';
}
