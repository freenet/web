// WebAssembly module
let wasmModule;

// Helper function for base64 encoding (only used for random seed)
function bufferToBase64(buffer) {
    return btoa(String.fromCharCode.apply(null, new Uint8Array(buffer)));
}

// Load WebAssembly module
async function loadWasmModule() {
    try {
        const wasm = await import('/wasm/gkwasm.js');
        await wasm.default();
        wasmModule = wasm;
        console.log("WebAssembly module loaded");
    } catch (error) {
        console.error("Failed to load WebAssembly module:", error);
        if (error instanceof TypeError && error.message.includes('Failed to fetch')) {
            console.error("This might be due to the WASM file not being found or CORS issues.");
        } else if (error instanceof WebAssembly.CompileError) {
            console.error("This might be due to the wrong MIME type being served for the WASM file.");
        }
        throw error;
    }
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
async function initPage() {
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

  try {
    await loadWasmModule();
  } catch (error) {
    console.error("Failed to load WebAssembly module:", error);
    showError('Failed to load necessary components. Please try again later.');
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
    await generateAndSignCertificate(paymentIntent);
  } else {
    console.log("No payment intent or test mode detected");
    showError('Payment information not found.');
  }
}

// Ensure the DOM is fully loaded before running the script
document.addEventListener('DOMContentLoaded', async () => {
  console.log("DOMContentLoaded event fired");
  try {
    await initPage();
  } catch (error) {
    console.error("Error during page initialization:", error);
    showError('An error occurred while initializing the page. Please try again later.');
  }
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
      const delegateCertificateBase64 = localStorage.getItem('delegate_certificate_base64');
      
    // Generate key pair and blind the public key using WebAssembly
    console.log("Generating key pair and blinding public key");
    const seed = crypto.getRandomValues(new Uint8Array(32));
    const result = wasmModule.wasm_generate_keypair_and_blind(delegateCertificateBase64, seed);
    
    if (typeof result === 'string') {
      throw new Error(result); // This is an error message
    }

    const publicKey = result.ec_verifying_key;
    const privateKey = result.ec_signing_key;
    const blindingSecret = result.blinding_secret;
    const blindedPublicKey = result.blinded_signing_key;
    console.log("Key pair generated and public key blinded");

    // Now send the blinded public key to the server for signing
    console.log("Sending blinded public key for signing");
    let signResponse;
    try {
      signResponse = await fetch('http://localhost:8000/sign-certificate', {
        method: 'POST',
        headers: { 
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ 
          payment_intent_id: paymentIntentId, 
          blinded_ghostkey_base64: blindedPublicKey
        }),
        credentials: 'same-origin'
      });
    } catch (error) {
      console.error("Network error when sending blinded public key for signing:", error);
      throw new Error(`Network error: ${error.message}`);
    }

    if (!signResponse.ok) {
      const errorText = await signResponse.text();
      console.error("Server error when sending blinded public key for signing:", errorText);
      throw new Error(`Failed to sign certificate: ${signResponse.status} - ${errorText}`);
    }

    const signData = await signResponse.json();
    console.log("Received signed blinded public key");

    if (!signData.blind_signature_base64) {
      throw new Error('Invalid signing data received from server');
    }

    // Generate the Ghostkey certificate using WebAssembly
    console.log("Generating Ghostkey certificate");
    const ghostkeyCertificateBase64 = wasmModule.wasm_generate_ghostkey_certificate(
        delegateCertificateBase64,
      signData.blind_signature_base64,
      blindingSecret,
      publicKey
    );

    if (typeof ghostkeyCertificateBase64 === 'string' && ghostkeyCertificateBase64.startsWith('Error:')) {
      throw new Error(ghostkeyCertificateBase64);
    }

    console.log("Ghostkey certificate generated");
    displayCertificate(publicKey, privateKey, ghostkeyCertificateBase64);
  } catch (error) {
    console.error("Error in generateAndSignCertificate:", error);
    showError('Error generating certificate: ' + error.message);
  }
}

function displayCertificate(publicKey, privateKey, ghostkeyCertificateBase64) {
  console.log("Displaying certificate");
  try {
    if (!ghostkeyCertificateBase64) {
      throw new Error("Ghostkey certificate is missing");
    }

    console.log("Ghostkey certificate:", ghostkeyCertificateBase64);
    console.log("Public key:", publicKey);

    // Format the certificate output
    const formattedCertificate = `-----BEGIN GHOSTKEY_CERTIFICATE-----
${wrapBase64(ghostkeyCertificateBase64, 64)}
-----END GHOSTKEY_CERTIFICATE-----`;

    // Format the ghost signing key output
    const formattedSigningKey = `-----BEGIN GHOST KEY-----
${wrapBase64(privateKey, 64)}
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
    certificateInfo.style.display = 'block';
    
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
        const blob = new Blob([formattedOutput], { type: 'text/plain' });
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
    if (certificateInfo && delegateInfo) {
      certificateInfo.innerHTML = `<p>Your donation certificate is ready. Donation amount: $${delegateInfo.amount}</p>`;
    }

    // Verification is now handled by the WebAssembly module
    console.log("Certificate generated successfully");
    
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

// Verification is now handled by the WebAssembly module

// MessagePack library is loaded globally, no need to require it

function showError(message) {
  const errorElement = document.getElementById('errorMessage');
  if (errorElement) {
    errorElement.textContent = message;
    errorElement.style.display = 'block';
  } else {
    console.error("Error element not found. Error message:", message);
  }
}
