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
      body: JSON.stringify({ paymentIntentId, blindedPublicKey })
    });

    if (!response.ok) {
      throw new Error(`Failed to sign certificate: ${response.status} ${response.statusText}`);
    }

    const data = await response.json();
    if (!data.blindSignature) {
      throw new Error('Invalid response from server: missing blindSignature');
    }
    const { blindSignature } = data;

    // Unblind the signature
    const unblindedSignature = ec.g.mul(ec.keyFromPrivate(blindSignature, 'hex').getPrivate())
      .add(ec.g.mul(blindingFactor).neg())
      .encode('hex');

    // Display the certificate
    document.getElementById('signedPublicKey').value = btoa(publicKey + '|' + unblindedSignature);
    document.getElementById('certificateSection').style.display = 'block';
    document.getElementById('certificate-info').style.display = 'none';

    // Set up download button
    document.getElementById('downloadCertificate').addEventListener('click', function() {
      const certificateData = {
        publicKey: publicKey,
        signature: unblindedSignature
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
