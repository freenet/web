/**
 * Encryption utilities using Web Crypto API
 * - PBKDF2 for key derivation (600k iterations per OWASP 2023)
 * - AES-256-GCM for encryption
 */

const PBKDF2_ITERATIONS = 600000;
const SALT_LENGTH = 16;
const IV_LENGTH = 12;

/**
 * Derive an AES-256-GCM key from a password using PBKDF2.
 */
export async function deriveKey(
  password: string,
  salt: Uint8Array | number[]
): Promise<CryptoKey> {
  const saltArray = salt instanceof Uint8Array ? salt : new Uint8Array(salt);

  const keyMaterial = await crypto.subtle.importKey(
    'raw',
    new TextEncoder().encode(password),
    'PBKDF2',
    false,
    ['deriveKey']
  );

  return crypto.subtle.deriveKey(
    {
      name: 'PBKDF2',
      salt: saltArray as BufferSource,
      iterations: PBKDF2_ITERATIONS,
      hash: 'SHA-256',
    },
    keyMaterial,
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt', 'decrypt']
  );
}

/**
 * Encrypt data with AES-256-GCM.
 */
export async function encryptData(
  key: CryptoKey,
  data: Uint8Array,
  iv: Uint8Array
): Promise<Uint8Array> {
  const ciphertext = await crypto.subtle.encrypt(
    { name: 'AES-GCM', iv: iv as BufferSource },
    key,
    data as BufferSource
  );
  return new Uint8Array(ciphertext);
}

/**
 * Decrypt data with AES-256-GCM.
 */
export async function decryptData(
  key: CryptoKey,
  ciphertext: Uint8Array | number[],
  iv: Uint8Array | number[]
): Promise<Uint8Array> {
  const ctArray =
    ciphertext instanceof Uint8Array ? ciphertext : new Uint8Array(ciphertext);
  const ivArray = iv instanceof Uint8Array ? iv : new Uint8Array(iv);

  const plaintext = await crypto.subtle.decrypt(
    { name: 'AES-GCM', iv: ivArray as BufferSource },
    key,
    ctArray as BufferSource
  );
  return new Uint8Array(plaintext);
}

/**
 * Generate cryptographically secure random salt.
 */
export function generateSalt(): Uint8Array {
  return crypto.getRandomValues(new Uint8Array(SALT_LENGTH));
}

/**
 * Generate cryptographically secure random IV.
 */
export function generateIv(): Uint8Array {
  return crypto.getRandomValues(new Uint8Array(IV_LENGTH));
}

/**
 * Convert Uint8Array to number array for JSON storage.
 */
export function toNumberArray(arr: Uint8Array): number[] {
  return Array.from(arr);
}

/**
 * Convert number array back to Uint8Array.
 */
export function fromNumberArray(arr: number[]): Uint8Array {
  return new Uint8Array(arr);
}
