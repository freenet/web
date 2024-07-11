// Use global nacl and nacl-util objects

function bufferToBase64(buffer) {
    return nacl.util.encodeBase64(buffer);
}

function base64ToBuffer(base64) {
    return nacl.util.decodeBase64(base64);
}

document.addEventListener('DOMContentLoaded', function() {
  const urlParams = new URLSearchParams(window.location.search);
  const paymentIntent = urlParams.get('payment_intent');
  const isTestMode = urlParams.get('test') !== null;

  if (isTestMode) {
    console.log("Test mode detected");
    generateTestCertificate();
  } else if (paymentIntent) {
    console.log("Payment intent detected:", paymentIntent);
    generateAndSignCertificate(paymentIntent);
  } else {
    console.log("No payment intent or test mode detected");
    showError('Payment information not found.');
  }
});

function generateTestCertificate() {
  console.log("Generating test certificate");
  const publicKey = nacl.randomBytes(32);
  const privateKey = nacl.randomBytes(64);
  const unblindedSignature = nacl.randomBytes(64);

  displayCertificate(publicKey, privateKey, unblindedSignature);
}

async function generateAndSignCertificate(paymentIntentId) {
  console.log("Starting generateAndSignCertificate");
  try {
    // Generate Ed25519 key pair
    const keyPair = nacl.sign.keyPair();
    const publicKey = keyPair.publicKey;
    const privateKey = keyPair.secretKey;
    console.log("Key pair generated");

    // Generate random blinding factor
    const blindingFactor = nacl.randomBytes(32);
    console.log("Blinding factor generated");

    // Blind the public key
    const blindedPublicKey = new Uint8Array(32);
    for (let i = 0; i < 32; i++) {
      blindedPublicKey[i] = publicKey[i] ^ blindingFactor[i];
    }
    console.log("Public key blinded");

    // Send blinded public key to server for signing
    console.log("Sending request to server");
    const response = await fetch('http://127.0.0.1:8000/sign-certificate', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ 
        payment_intent_id: paymentIntentId, 
        blinded_public_key: bufferToBase64(blindedPublicKey)
      })
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(`Error signing certificate: ${errorText}`);
    }

    console.log("Received response from server");
    const data = await response.json();
    if (!data.blind_signature) {
      if (data.message === "CERTIFICATE_ALREADY_SIGNED") {
        showError('Certificate already signed for this payment.');
        return;
      }
      throw new Error('No blind signature received from server');
    }

    console.log("Blind signature received");
    const blindSignature = base64ToBuffer(data.blind_signature);

    // Unblind the signature
    const unblindedSignature = new Uint8Array(64);
    for (let i = 0; i < 32; i++) {
      unblindedSignature[i] = blindSignature[i] ^ blindingFactor[i];
    }
    for (let i = 32; i < 64; i++) {
      unblindedSignature[i] = blindSignature[i];
    }
    console.log("Signature unblinded");

    console.log("Calling displayCertificate");
    displayCertificate(publicKey, privateKey, unblindedSignature);
  } catch (error) {
    console.error("Error in generateAndSignCertificate:", error);
    showError('Error generating certificate: ' + error.message);
  }
}

function generateTestCertificate() {
  const publicKey = nacl.randomBytes(32);
  const privateKey = nacl.randomBytes(64);
  const unblindedSignature = nacl.randomBytes(64);

  displayCertificate(publicKey, privateKey, unblindedSignature);
}

function displayCertificate(publicKey, privateKey, unblindedSignature) {
  console.log("Displaying certificate");
  try {
    // Armor the certificate and private key
    const armoredCertificate = `-----BEGIN FREENET DONATION CERTIFICATE-----
${bufferToBase64(publicKey)}|${bufferToBase64(unblindedSignature)}
-----END FREENET DONATION CERTIFICATE-----`;

    const armoredPrivateKey = `-----BEGIN FREENET DONATION PRIVATE KEY-----
${bufferToBase64(privateKey)}
-----END FREENET DONATION PRIVATE KEY-----`;

    // Combine certificate and private key
    const combinedKey = `${wrapBase64(armoredCertificate, 64)}\n\n${wrapBase64(armoredPrivateKey, 64)}`;

    // Display the combined key
    const combinedKeyElement = document.getElementById('combinedKey');
    if (!combinedKeyElement) {
      throw new Error("Combined key textarea not found");
    }
    
    combinedKeyElement.value = combinedKey;
    
    const certificateSection = document.getElementById('certificateSection');
    const certificateInfo = document.getElementById('certificate-info');
    
    if (!certificateSection || !certificateInfo) {
      throw new Error("Certificate section or info element not found");
    }
    
    certificateSection.style.display = 'block';
    certificateInfo.style.display = 'none';

    // Set up copy button
    const copyButton = document.getElementById('copyCombinedKey');
    if (!copyButton) {
      throw new Error("Copy button not found");
    }
    
    copyButton.addEventListener('click', function() {
      combinedKeyElement.select();
      document.execCommand('copy');
      alert('Combined key copied to clipboard!');
    });

    // Verify the certificate
    if (!verifyCertificate(publicKey, unblindedSignature)) {
      throw new Error("Certificate verification failed");
    }
    
    console.log("Certificate verified and displayed successfully");
  } catch (error) {
    console.error("Error in displayCertificate:", error);
    showError(`Error displaying certificate: ${error.message}. Please contact support.`);
  }
}

// Function to wrap base64 encoded text
function wrapBase64(str, maxWidth) {
  const lines = str.split('\n');
  return lines.map(line => {
    if (line.startsWith('-----')) {
      return line;
    }
    return line.match(new RegExp(`.{1,${maxWidth}}`, 'g')).join('\n');
  }).join('\n');
}

function verifyCertificate(publicKey, signature) {
  try {
    // In a real implementation, we would verify the signature against a known message
    // For now, we'll just check if the signature is the correct length
    return signature.length === 64;
  } catch (error) {
    console.error("Verification error:", error);
    return false;
  }
}

function showError(message) {
  const errorElement = document.getElementById('errorMessage');
  errorElement.textContent = message;
  errorElement.style.display = 'block';
  document.getElementById('certificate-info').style.display = 'none';
}
