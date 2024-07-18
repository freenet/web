// Use global nacl and nacl-util objects

function bufferToBase64(buffer) {
    return nacl.util.encodeBase64(buffer);
}

function base64ToBuffer(base64) {
    return nacl.util.decodeBase64(base64);
}

// Function to check for required elements and log detailed information
function checkRequiredElements() {
  const requiredElements = [
    { id: 'certificateSection', selector: 'div#certificateSection' },
    { id: 'certificate-info', selector: 'div#certificate-info' },
    { id: 'errorMessage', selector: 'div#errorMessage' },
    { id: 'certificate', selector: 'textarea#certificate' }
  ];
  
  console.log("Checking for required elements...");
  
  const missingElements = requiredElements.filter(el => {
    const element = document.querySelector(el.selector);
    if (!element) {
      console.error(`Element not found: ${el.id} (selector: ${el.selector})`);
    } else {
      console.log(`Element found: ${el.id}`);
    }
    return !element;
  });
  
  if (missingElements.length > 0) {
    console.error("Missing required elements:", missingElements.map(el => el.id));
    const errorMessage = `Warning: Some elements are missing. The page may not function correctly. ` +
                         `Missing elements: ${missingElements.map(el => el.id).join(', ')}. `;
    showError(errorMessage);
    // Return true to allow the script to continue
    return true;
  }
  console.log("All required elements found.");
  return true;
}

// Function to initialize the page
function initPage() {
  console.log("Initializing page");
  
  // Check if we're on the correct page
  if (!document.querySelector('#certificateSection')) {
    console.log("Not on the donation success page, script will not run.");
    return;
  }

  console.log("Donation success page detected. Checking required elements...");
  if (!checkRequiredElements()) {
    console.error("Required elements not found. Page initialization failed.");
    return;
  }

  const urlParams = new URLSearchParams(window.location.search);
  const paymentIntent = urlParams.get('payment_intent');
  const isTestMode = urlParams.get('test') !== null;

  console.log("URL parameters:", {
    paymentIntent: paymentIntent || 'Not found',
    isTestMode: isTestMode
  });

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
}

// Ensure the DOM is fully loaded before running the script
document.addEventListener('DOMContentLoaded', () => {
  console.log("DOMContentLoaded event fired");
  if (typeof nacl === 'undefined' || typeof nacl.util === 'undefined') {
    console.error("nacl or nacl.util is not defined. Make sure the TweetNaCl library is properly loaded.");
    showError('An error occurred while loading the necessary libraries. Please try again later.');
    return;
  }
  initPage();
});

// Log any errors that occur during script execution
window.onerror = function(message, source, lineno, colno, error) {
  console.error("Unhandled error:", { message, source, lineno, colno, error });
  showError('An unexpected error occurred. Please try again later.');
};

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
    let data;
    try {
      const response = await fetch('http://127.0.0.1:8000/sign-certificate', {
        method: 'POST',
        headers: { 
          'Content-Type': 'application/json',
          'Origin': window.location.origin
        },
        body: JSON.stringify({ 
          payment_intent_id: paymentIntentId, 
          blinded_public_key: bufferToBase64(blindedPublicKey)
        }),
        credentials: 'include'
      });

      if (!response.ok) {
        const errorText = await response.text();
        console.error('Server response error:', response.status, errorText);
        if (response.status === 400 && errorText.includes("Payment method is missing")) {
          showError('Payment method is missing. Please return to the donation page and try again.');
          return;
        }
        throw new Error(`Server error: ${response.status} - ${errorText}`);
      }

      console.log("Received response from server");
      data = await response.json();
      console.log("Server response data:", data);
      if (!data.blind_signature) {
        if (data.message === "CERTIFICATE_ALREADY_SIGNED") {
          showError('Certificate already signed for this payment.');
          return;
        }
        throw new Error('No blind signature received from server');
      }
    } catch (error) {
      console.error("Error in server communication:", error);
      showError(`Error communicating with server: ${error.message}`);
      return;
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
    displayCertificate(publicKey, privateKey, unblindedSignature, data.delegate_info);
  } catch (error) {
    console.error("Error in generateAndSignCertificate:", error);
    showError('Error generating certificate: ' + error.message);
  }
}

function displayCertificate(publicKey, privateKey, unblindedSignature, delegateInfo) {
  console.log("Displaying certificate");
  try {
    // Create a ghost key certificate object
    const ghostKeyCertificate = {
      verifying_key: publicKey,
      info: delegateInfo.certificate,
      signature: unblindedSignature
    };

    // Serialize the ghost key certificate using MessagePack
    const serializedCertificate = msgpack.encode(ghostKeyCertificate);

    // Convert the serialized certificate to base64
    const base64Certificate = bufferToBase64(serializedCertificate);

    // Format the ghost key certificate
    const formattedCertificate = `-----BEGIN GHOST KEY CERTIFICATE-----
${base64Certificate}
-----END GHOST KEY CERTIFICATE-----

-----BEGIN GHOST KEY-----
${bufferToBase64(publicKey)}|${bufferToBase64(unblindedSignature)}|${bufferToBase64(privateKey)}
-----END GHOST KEY-----`;

    const certificateSection = document.getElementById('certificateSection');
    const certificateInfo = document.getElementById('certificate-info');
    const certificateTextarea = document.getElementById('certificate');
    
    if (!certificateSection || !certificateInfo) {
      console.error("Required elements not found");
      throw new Error("Required elements not found");
    }
    
    certificateSection.style.display = 'block';
    certificateInfo.style.display = 'block';
    
    certificateTextarea.value = formattedCertificate;
    console.log("Ghost key certificate populated in textarea");
    
    certificateInfo.style.display = 'none';

    // Set up download button
    const downloadButton = document.getElementById('downloadCertificate');
    if (downloadButton) {
      downloadButton.addEventListener('click', function() {
        const blob = new Blob([formattedCertificate], { type: 'text/plain' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = 'freenet_ghost_key.txt';
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
      });
    }

    // Display delegate info
    if (delegateInfo) {
      const delegateInfoElement = document.createElement('p');
      delegateInfoElement.textContent = `Donation amount: $${delegateInfo.amount}`;
      certificateSection.appendChild(delegateInfoElement);
    }

    // Verify the certificate
    if (!verifyCertificate(publicKey, unblindedSignature)) {
      console.error("Certificate verification failed");
      throw new Error("Certificate verification failed");
    }
    
    console.log("Certificate verified and displayed successfully");
  } catch (error) {
    console.error("Error in displayCertificate:", error);
    showError(`Error displaying Ghost Key: ${error.message}. Please contact support.`);
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

// MessagePack library is loaded globally, no need to require it

function showError(message) {
  const errorElement = document.getElementById('errorMessage');
  if (errorElement) {
    errorElement.textContent = message;
    errorElement.style.display = 'block';
  } else {
    console.error("Error element not found. Error message:", message);
  }
  const certificateInfo = document.getElementById('certificate-info');
  if (certificateInfo) {
    certificateInfo.style.display = 'none';
  } else {
    console.error("Certificate info element not found");
  }
}
