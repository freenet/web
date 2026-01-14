/**
 * Crypto service wrapping the gkwasm WASM module
 */
import type { AuthRequest, AuthResponse, VerifyResult } from '../shared/types';

// WASM module instance
let wasmModule: any = null;

/**
 * Initialize the WASM module.
 */
export async function initWasm(): Promise<void> {
  if (wasmModule) return;

  const wasmUrl = chrome.runtime.getURL('wasm/gkwasm.js');
  const wasm = await import(/* webpackIgnore: true */ wasmUrl);
  await wasm.default(chrome.runtime.getURL('wasm/gkwasm_bg.wasm'));
  wasmModule = wasm;
}

/**
 * Sign a message with a ghostkey.
 */
export async function signMessage(
  certificate: string,
  signingKey: string,
  message: string
): Promise<string> {
  if (!wasmModule) {
    throw new Error('WASM module not initialized');
  }

  const messageBytes = new TextEncoder().encode(message);
  const result = wasmModule.wasm_sign_message(
    certificate,
    signingKey,
    Array.from(messageBytes)
  );

  if (typeof result === 'string' && result.startsWith('-----BEGIN')) {
    return result;
  }

  throw new Error(result || 'Failed to sign message');
}

/**
 * Sign a structured authentication request.
 */
export async function signAuthRequest(
  certificate: string,
  signingKey: string,
  request: Omit<AuthRequest, 'type' | 'timestamp'>
): Promise<AuthResponse> {
  if (!wasmModule) {
    throw new Error('WASM module not initialized');
  }

  // Build canonical auth request
  const authRequest: AuthRequest = {
    type: 'auth',
    origin: request.origin,
    contractAddress: request.contractAddress,
    challenge: request.challenge,
    timestamp: Date.now(),
    purpose: request.purpose,
  };

  // Serialize to canonical JSON (sorted keys)
  const canonicalJson = JSON.stringify(authRequest, Object.keys(authRequest).sort());
  const messageBytes = new TextEncoder().encode(canonicalJson);

  const signedAuth = wasmModule.wasm_sign_message(
    certificate,
    signingKey,
    Array.from(messageBytes)
  );

  if (typeof signedAuth !== 'string' || !signedAuth.startsWith('-----BEGIN')) {
    throw new Error(signedAuth || 'Failed to sign auth request');
  }

  return {
    signedAuth,
    certificate,
  };
}

/**
 * Verify a signed message.
 */
export async function verifySignedMessage(
  signedMessage: string,
  masterVerifyingKey?: string
): Promise<VerifyResult> {
  if (!wasmModule) {
    throw new Error('WASM module not initialized');
  }

  try {
    const result = wasmModule.wasm_verify_signed_message(
      signedMessage,
      masterVerifyingKey || null
    );

    return {
      valid: result.valid,
      info: result.info,
      message: new Uint8Array(result.message),
    };
  } catch (e) {
    return {
      valid: false,
      info: String(e),
      message: new Uint8Array(),
    };
  }
}

/**
 * Extract the certificate info (donation amount, etc.) from a certificate.
 */
export async function getCertificateInfo(certificate: string): Promise<string> {
  if (!wasmModule) {
    throw new Error('WASM module not initialized');
  }

  // Create a dummy signed message to extract info via verification
  // This is a workaround - ideally we'd have a dedicated function
  // For now, just return empty - we can add a dedicated WASM function later
  return '';
}
