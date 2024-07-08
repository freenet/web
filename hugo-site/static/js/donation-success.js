function bufferToBase64(buffer) {
    let binary = '';
    const bytes = new Uint8Array(buffer);
    const len = bytes.byteLength;
    for (let i = 0; i < len; i++) {
        binary += String.fromCharCode(bytes[i]);
    }
    return window.btoa(binary).replace(/=+$/, '');
}

function base64ToBuffer(base64) {
    // Add padding if necessary
    const paddedBase64 = base64.padEnd(base64.length + (4 - base64.length % 4) % 4, '=');
    const binary = window.atob(paddedBase64);
    const len = binary.length;
    const bytes = new Uint8Array(len);
    for (let i = 0; i < len; i++) {
        bytes[i] = binary.charCodeAt(i);
    }
    return bytes.buffer;
}

document.addEventListener('DOMContentLoaded', function() {
  const urlParams = new URLSearchParams(window.location.search);
  const paymentIntent = urlParams.get('payment_intent');

  if (paymentIntent) {
    generateAndSignCertificate(paymentIntent);
  } else {
    showError('Payment information not found.');
  }
});

function bufferToBase64(buffer) {
    let binary = '';
    const bytes = new Uint8Array(buffer);
    const len = bytes.byteLength;
    for (let i = 0; i < len; i++) {
        binary += String.fromCharCode(bytes[i]);
    }
    return window.btoa(binary);
}

async function generateAndSignCertificate(paymentIntentId) {
  try {
    // Generate a key pair
    const keyPair = await window.crypto.subtle.generateKey(
      {
        name: "ECDSA",
        namedCurve: "P-256"
      },
      true,
      ["sign", "verify"]
    );

    // Export the public key
    const publicKeyBuffer = await window.crypto.subtle.exportKey("raw", keyPair.publicKey);
    const publicKey = bufferToBase64(publicKeyBuffer);

    // Generate a random blinding factor
    const blindingFactor = await window.crypto.subtle.generateKey(
      {
        name: "ECDH",
        namedCurve: "P-256"
      },
      true,
      ["deriveBits"]
    );

    // Blind the public key
    const blindedPublicKey = await blindPublicKey(publicKeyBuffer, blindingFactor);

    // Send blinded public key to server for signing
    const response = await fetch('http://127.0.0.1:8000/sign-certificate', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ payment_intent_id: paymentIntentId, blinded_public_key: bufferToBase64(blindedPublicKey) })
    });

    if (!response.ok) {
      const contentType = response.headers.get('content-type');
      if (contentType && contentType.includes('application/json')) {
        const errorData = await response.json();
        throw new Error(`Error signing certificate: ${errorData.message || 'Unknown error'}`);
      } else {
        const errorText = await response.text();
        throw new Error(`Error signing certificate: ${errorText}`);
      }
    }

    const contentType = response.headers.get('content-type');
    let data;
    if (contentType && contentType.includes('application/json')) {
      data = await response.json();
    } else {
      const errorText = await response.text();
      throw new Error(`Unexpected response format: ${errorText}`);
    }
    if (!data.blind_signature) {
      throw new Error(`Failed to sign certificate: ${data.message || 'Unknown error'}`);
    }
    const blindSignature = base64ToBuffer(data.blind_signature);

    // Unblind the signature
    const unblindedSignature = await unblindSignature(blindSignature, blindingFactor);

    // Armor the certificate and private key
    const armoredCertificate = `-----BEGIN FREENET DONATION CERTIFICATE-----
${publicKey}|${bufferToBase64(unblindedSignature)}
-----END FREENET DONATION CERTIFICATE-----`;

    const privateKeyBuffer = await window.crypto.subtle.exportKey("pkcs8", keyPair.privateKey);
    const armoredPrivateKey = `-----BEGIN FREENET DONATION PRIVATE KEY-----
${bufferToBase64(privateKeyBuffer)}
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
    if (await verifyCertificate(keyPair.publicKey, unblindedSignature)) {
      console.log("Certificate verified successfully");
    } else {
      console.error("Certificate verification failed");
      showError('Certificate verification failed. Please contact support.');
    }
  } catch (error) {
    showError('Error generating certificate: ' + error.message);
  }
}

async function blindPublicKey(publicKey, blindingFactor) {
  const publicKeyPoint = await window.crypto.subtle.importKey(
    "raw",
    publicKey,
    {
      name: "ECDH",
      namedCurve: "P-256"
    },
    true,
    []
  );

  const blindingFactorPoint = await window.crypto.subtle.importKey(
    "raw",
    await window.crypto.subtle.exportKey("raw", blindingFactor.publicKey),
    {
      name: "ECDH",
      namedCurve: "P-256"
    },
    true,
    []
  );

  // Perform point addition using the Web Crypto API
  const blindedPublicKey = await window.crypto.subtle.deriveBits(
    {
      name: "ECDH",
      public: publicKeyPoint
    },
    blindingFactor.privateKey,
    256
  );

  return new Uint8Array(blindedPublicKey);
}

async function unblindSignature(blindSignature, blindingFactor) {
  const blindingFactorPoint = await window.crypto.subtle.exportKey("raw", blindingFactor.privateKey);
  
  // Perform point subtraction (this is a simplified representation, actual ECC operations are more complex)
  const unblindedSignature = new Uint8Array(blindSignature.byteLength);
  for (let i = 0; i < blindSignature.byteLength; i++) {
    unblindedSignature[i] = blindSignature[i] ^ blindingFactorPoint[i];
  }

  return unblindedSignature;
}

async function verifyCertificate(publicKey, signature) {
  try {
    // Verify the signature against a known message
    const result = await window.crypto.subtle.verify(
      {
        name: "ECDSA",
        hash: {name: "SHA-256"},
      },
      publicKey,
      signature,
      new Uint8Array(0)
    );
    return result;
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

function bufferToHex(buffer) {
  return Array.from(new Uint8Array(buffer))
    .map(b => b.toString(16).padStart(2, '0'))
    .join('');
}

function hexToBuffer(hex) {
  return new Uint8Array(hex.match(/.{1,2}/g).map(byte => parseInt(byte, 16)));
}
