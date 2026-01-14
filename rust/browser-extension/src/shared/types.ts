/**
 * Root vault structure stored in chrome.storage.local
 */
export interface StoredVault {
  version: number;
  salt: number[];
  verification: number[];
  verificationIv: number[];
  keys: Record<string, EncryptedKey>;
  activeKeyId: string | null;
  unlockMethod: 'password' | 'passkey' | 'both';
  passkeyCredentialId?: string;
}

/**
 * Individual encrypted key entry
 */
export interface EncryptedKey {
  id: string;
  label: string;
  createdAt: number;
  encryptedData: number[];
  iv: number[];
}

/**
 * Decrypted key data (never stored in plaintext)
 */
export interface GhostkeyEntry {
  id: string;
  label: string;
  createdAt: number;
  certificate: string;
  signingKey: string;
}

/**
 * Metadata returned to UI (no secrets)
 */
export interface KeyMetadata {
  id: string;
  label: string;
  createdAt: number;
  isActive: boolean;
}

/**
 * Structured authentication request for websites
 */
export interface AuthRequest {
  type: 'auth';
  origin: string;
  contractAddress?: string;
  challenge: string;
  timestamp: number;
  purpose: string;
}

/**
 * Response from signing an auth request
 */
export interface AuthResponse {
  signedAuth: string;
  certificate: string;
}

/**
 * Result from verifying a signed message
 */
export interface VerifyResult {
  valid: boolean;
  info: string;
  message: Uint8Array;
}
