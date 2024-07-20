// Use global nacl and nacl-util objects

function bufferToBase64(buffer) {
    return nacl.util.encodeBase64(buffer);
}

function base64ToBuffer(base64) {
    return nacl.util.decodeBase64(base64);
}

// Helper function to encode ArrayBuffer to base64
function arrayBufferToBase64(buffer) {
    return btoa(String.fromCharCode.apply(null, new Uint8Array(buffer)));
}

// Function to check for required elements and log detailed information
function checkRequiredElements() {
  const requiredElements = [
    { id: 'certificateSection', selector: 'div#certificateSection' },
    { id: 'certificate-info', selector: 'div#certificate-info' },
    { id: 'errorMessage', selector: 'div#errorMessage' },
    { id: 'combinedKey', selector: 'textarea#combinedKey' }
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

    const blindingFactor = nacl.randomBytes(32);
    console.log("Blinding factor generated:", bufferToBase64(blindingFactor));

    // Convert the public key to a curve point
    const publicKeyPoint = nacl.scalarMult.base(publicKey);

    // Blind the public key (message)
    const blindedPublicKey = nacl.scalarMult(blindingFactor, publicKeyPoint);
    console.log("Public key blinded");
    console.log("Original public key:", bufferToBase64(publicKey));
    console.log("Blinded public key:", bufferToBase64(blindedPublicKey));

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
    const blindingFactorInverse = nacl.scalarMult.base(blindingFactor);
    console.log("Blinding factor:", bufferToBase64(blindingFactor));
    console.log("Blinding factor inverse:", bufferToBase64(blindingFactorInverse));
    console.log("Blinding factor inverse length:", blindingFactorInverse.length);
    console.log("Blind signature:", bufferToBase64(blindSignature));
    console.log("Blinding factor length:", blindingFactor.length);
    console.log("Blinding factor inverse length:", blindingFactorInverse.length);
    console.log("Blind signature length:", blindSignature.length);

    // Split the combined signature into its components
    const signature = blindSignature.slice(0, 64);
    const nonce = blindSignature.slice(64);

    // Unblind the signature
    if (blindingFactorInverse.length !== 32 || signature.length !== 32) {
        throw new Error(`Invalid sizes for scalar multiplication: blindingFactorInverse length = ${blindingFactorInverse.length}, signature length = ${signature.length}`);
    }
    const unblindedSignature = nacl.scalarMult(blindingFactorInverse, signature);
    console.log("Signature unblinded");
    console.log("Unblinded signature:", bufferToBase64(unblindedSignature));

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
    if (!delegateInfo || !delegateInfo.certificate) {
      throw new Error("Delegate certificate is missing");
    }

    console.log("Delegate certificate:", delegateInfo.certificate);
    console.log("Public key:", bufferToBase64(publicKey));
    console.log("Unblinded signature:", bufferToBase64(unblindedSignature));
    console.log("Delegate info:", delegateInfo);

    let ghostKeyCertificate, serializedCertificate, base64Certificate;

    try {
      // Create a ghost key certificate object matching the Rust structure
      let decodedDelegateCertificate;
      try {
        // Extract the base64 content from the armored format
        const base64Content = delegateInfo.certificate
          .replace(/-----BEGIN DELEGATE CERTIFICATE-----/, '')
          .replace(/-----END DELEGATE CERTIFICATE-----/, '')
          .replace(/\s/g, '');
        decodedDelegateCertificate = base64ToBuffer(base64Content);
        console.log("Decoded delegate certificate:", decodedDelegateCertificate);
        
        // Ensure decodedDelegateCertificate is a Uint8Array
        if (!(decodedDelegateCertificate instanceof Uint8Array)) {
          decodedDelegateCertificate = new Uint8Array(decodedDelegateCertificate);
        }
      } catch (decodeError) {
        console.error("Error decoding armored delegate certificate:", decodeError);
        throw new Error(`Failed to decode armored delegate certificate: ${decodeError.message}`);
      }

      // Create the GhostkeyCertificate object
      const ghostKeyCertificate = {
        version: 1,
        delegate_certificate: Array.from(new Uint8Array(decodedDelegateCertificate)),
        ghostkey_verifying_key: Array.from(new Uint8Array(publicKey)),
        signature: Array.from(new Uint8Array(unblindedSignature.buffer))
      };
      console.log("Ghost key certificate:", ghostKeyCertificate);
      console.log("Ghost key certificate object created:", ghostKeyCertificate);

      // Create the GhostkeySigningData object
      const ghostkeySigningData = {
        version: 1,
        delegate_certificate: Array.from(new Uint8Array(decodedDelegateCertificate)),
        ghostkey_verifying_key: Array.from(new Uint8Array(publicKey))
      };
      console.log("Ghost key signing data object created:", ghostkeySigningData);

      // Serialize the GhostkeyCertificate using MessagePack
      const serializedCertificate = msgpack.encode(ghostKeyCertificate);
      console.log("GhostkeyCertificate serialized:", serializedCertificate);

      // Convert the serialized certificate to base64
      const base64Certificate = bufferToBase64(serializedCertificate);
      console.log("Serialized certificate converted to base64:", base64Certificate);

      // Format the certificate output
      const formattedCertificate = `-----BEGIN GHOSTKEY CERTIFICATE-----
${wrapBase64(base64Certificate, 64)}
-----END GHOSTKEY CERTIFICATE-----`;

      // Convert the ghost signing key (privateKey) to base64
      const base64SigningKey = bufferToBase64(privateKey);

      // Format the ghost signing key output
      const formattedSigningKey = `-----BEGIN GHOST KEY-----
${wrapBase64(base64SigningKey, 64)}
-----END GHOST KEY-----`;

      // Combine the certificate and signing key
      const formattedOutput = `${formattedCertificate}\n\n${formattedSigningKey}`;

      console.log("Ghost Key Certificate and Signing Key created successfully");

      const certificateSection = document.getElementById('certificateSection');
      const certificateInfo = document.getElementById('certificate-info');
      const combinedKeyTextarea = document.getElementById('combinedKey');
      
      if (!certificateSection || !certificateInfo || !combinedKeyTextarea) {
        console.error("Required elements not found");
        throw new Error("Required elements not found");
      }
      
      certificateSection.style.display = 'block';
      certificateInfo.style.display = 'none';
      
      combinedKeyTextarea.value = formattedOutput;
      console.log("Ghost Key populated in textarea");

    } catch (encodingError) {
      console.error("Error in GhostKey encoding:", encodingError);
      throw new Error(`GhostKey encoding failed: ${encodingError.message}`);
    }
    
    if (!certificateSection || !certificateInfo || !combinedKeyTextarea) {
      console.error("Required elements not found");
      throw new Error("Required elements not found");
    }
    
    certificateSection.style.display = 'block';
    certificateInfo.style.display = 'none';
    
    combinedKeyTextarea.value = formattedOutput;
    console.log("Ghost Key populated in textarea");

    // Set up copy button
    const copyButton = document.getElementById('copyCombinedKey');
    if (copyButton) {
      copyButton.addEventListener('click', function() {
        combinedKeyTextarea.select();
        document.execCommand('copy');
        alert('Ghost Key copied to clipboard!');
      });
    }

    // Set up download button
    const downloadButton = document.getElementById('downloadCertificate');
    if (downloadButton) {
      downloadButton.addEventListener('click', function() {
        const blob = new Blob([combinedKey], { type: 'text/plain' });
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
      const delegateInfoElement = document.getElementById('certificate-info');
      if (delegateInfoElement) {
        delegateInfoElement.innerHTML = `<p>Your donation certificate is ready. Donation amount: $${delegateInfo.amount}</p>`;
        delegateInfoElement.style.display = 'block';
      }
    } else {
      throw new Error("Delegate information is missing");
    }

    // Verify the certificate
    if (!verifyCertificate(publicKey, unblindedSignature, decodedDelegateCertificate)) {
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

function verifyCertificate(publicKey, signature, delegateCertificate) {
  try {
    // Create the GhostkeySigningData object
    const ghostkeySigningData = {
      version: 1,
      delegate_certificate: Array.from(new Uint8Array(delegateCertificate)),
      ghostkey_verifying_key: Array.from(new Uint8Array(publicKey))
    };

    // Serialize the GhostkeySigningData using MessagePack
    const message = msgpack.encode(ghostkeySigningData);

    // Verify the signature using TweetNaCl
    return nacl.sign.detached.verify(message, signature, publicKey);
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
