/**
 * Storage service for encrypted ghostkey vault
 */
import {
  deriveKey,
  encryptData,
  decryptData,
  generateSalt,
  generateIv,
  toNumberArray,
  fromNumberArray,
} from '../shared/encryption';
import type {
  StoredVault,
  EncryptedKey,
  GhostkeyEntry,
  KeyMetadata,
} from '../shared/types';

const STORAGE_KEY = 'ghostkey_vault';
const VERIFICATION_TOKEN = 'FREENET_GHOSTKEY_VAULT_V1';

export class StorageService {
  private derivedKey: CryptoKey | null = null;
  private sessionActive: boolean = false;

  /**
   * Check if vault exists
   */
  async vaultExists(): Promise<boolean> {
    const stored = await chrome.storage.local.get([STORAGE_KEY]);
    return !!stored[STORAGE_KEY];
  }

  /**
   * Check if vault is currently unlocked
   */
  isUnlocked(): boolean {
    return this.sessionActive && this.derivedKey !== null;
  }

  /**
   * Unlock the vault with password.
   */
  async unlock(password: string): Promise<boolean> {
    const stored = await chrome.storage.local.get([STORAGE_KEY]);

    if (!stored[STORAGE_KEY]) {
      // First time - create new vault
      return this.initializeVault(password);
    }

    const vault: StoredVault = stored[STORAGE_KEY];
    try {
      this.derivedKey = await deriveKey(password, vault.salt);
      // Test decryption with verification data
      const decrypted = await decryptData(
        this.derivedKey,
        vault.verification,
        vault.verificationIv
      );
      const token = new TextDecoder().decode(decrypted);
      if (token !== VERIFICATION_TOKEN) {
        throw new Error('Invalid verification token');
      }
      this.sessionActive = true;
      return true;
    } catch {
      this.derivedKey = null;
      this.sessionActive = false;
      return false;
    }
  }

  /**
   * Lock the vault, clearing the derived key from memory.
   */
  lock(): void {
    this.derivedKey = null;
    this.sessionActive = false;
  }

  /**
   * Import a ghostkey from PEM content.
   */
  async importKey(pemContent: string, label: string): Promise<string> {
    if (!this.derivedKey) throw new Error('Vault is locked');

    const certificate = this.extractBlock(pemContent, 'GHOSTKEY_CERTIFICATE_V1');
    const signingKey = this.extractBlock(pemContent, 'SIGNING_KEY_V1');

    if (!certificate || !signingKey) {
      throw new Error('Invalid PEM content: missing certificate or signing key');
    }

    const keyId = crypto.randomUUID();
    const entry: GhostkeyEntry = {
      id: keyId,
      label,
      createdAt: Date.now(),
      certificate,
      signingKey,
    };

    const iv = generateIv();
    const encryptedData = await encryptData(
      this.derivedKey,
      new TextEncoder().encode(JSON.stringify(entry)),
      iv
    );

    const stored = await chrome.storage.local.get([STORAGE_KEY]);
    const vault: StoredVault = stored[STORAGE_KEY];
    vault.keys[keyId] = {
      id: keyId,
      label,
      createdAt: entry.createdAt,
      encryptedData: toNumberArray(encryptedData),
      iv: toNumberArray(iv),
    };

    // Set as active if it's the first key
    if (Object.keys(vault.keys).length === 1) {
      vault.activeKeyId = keyId;
    }

    await chrome.storage.local.set({ [STORAGE_KEY]: vault });
    return keyId;
  }

  /**
   * Delete a key from the vault.
   */
  async deleteKey(keyId: string): Promise<void> {
    const stored = await chrome.storage.local.get([STORAGE_KEY]);
    const vault: StoredVault = stored[STORAGE_KEY];

    if (!vault.keys[keyId]) {
      throw new Error('Key not found');
    }

    delete vault.keys[keyId];

    // If deleted key was active, pick another or set null
    if (vault.activeKeyId === keyId) {
      const remainingKeys = Object.keys(vault.keys);
      vault.activeKeyId = remainingKeys.length > 0 ? remainingKeys[0] : null;
    }

    await chrome.storage.local.set({ [STORAGE_KEY]: vault });
  }

  /**
   * Get list of keys (metadata only, not decrypted).
   */
  async getKeyList(): Promise<KeyMetadata[]> {
    const stored = await chrome.storage.local.get([STORAGE_KEY]);
    if (!stored[STORAGE_KEY]) return [];

    const vault: StoredVault = stored[STORAGE_KEY];
    return Object.values(vault.keys).map((k: EncryptedKey) => ({
      id: k.id,
      label: k.label,
      createdAt: k.createdAt,
      isActive: k.id === vault.activeKeyId,
    }));
  }

  /**
   * Set the active key.
   */
  async setActiveKey(keyId: string): Promise<void> {
    const stored = await chrome.storage.local.get([STORAGE_KEY]);
    const vault: StoredVault = stored[STORAGE_KEY];

    if (!vault.keys[keyId]) {
      throw new Error('Key not found');
    }

    vault.activeKeyId = keyId;
    await chrome.storage.local.set({ [STORAGE_KEY]: vault });
  }

  /**
   * Get decrypted key entry.
   */
  async getDecryptedKey(keyId: string): Promise<GhostkeyEntry> {
    if (!this.derivedKey) throw new Error('Vault is locked');

    const stored = await chrome.storage.local.get([STORAGE_KEY]);
    const vault: StoredVault = stored[STORAGE_KEY];
    const encryptedKey = vault.keys[keyId];

    if (!encryptedKey) {
      throw new Error('Key not found');
    }

    const decrypted = await decryptData(
      this.derivedKey,
      encryptedKey.encryptedData,
      encryptedKey.iv
    );

    return JSON.parse(new TextDecoder().decode(decrypted));
  }

  /**
   * Get the active key's decrypted entry.
   */
  async getActiveKey(): Promise<GhostkeyEntry | null> {
    const stored = await chrome.storage.local.get([STORAGE_KEY]);
    if (!stored[STORAGE_KEY]) return null;

    const vault: StoredVault = stored[STORAGE_KEY];
    if (!vault.activeKeyId) return null;

    return this.getDecryptedKey(vault.activeKeyId);
  }

  /**
   * Export a key as PEM content.
   */
  async exportKey(keyId: string): Promise<string> {
    const entry = await this.getDecryptedKey(keyId);
    return `${entry.certificate}\n${entry.signingKey}`;
  }

  private async initializeVault(password: string): Promise<boolean> {
    const salt = generateSalt();
    this.derivedKey = await deriveKey(password, salt);

    const verificationData = new TextEncoder().encode(VERIFICATION_TOKEN);
    const verificationIv = generateIv();
    const verification = await encryptData(
      this.derivedKey,
      verificationData,
      verificationIv
    );

    const vault: StoredVault = {
      version: 1,
      salt: toNumberArray(salt),
      verification: toNumberArray(verification),
      verificationIv: toNumberArray(verificationIv),
      keys: {},
      activeKeyId: null,
      unlockMethod: 'password',
    };

    await chrome.storage.local.set({ [STORAGE_KEY]: vault });
    this.sessionActive = true;
    return true;
  }

  private extractBlock(pem: string, blockType: string): string | null {
    const regex = new RegExp(
      `-----BEGIN ${blockType}-----[\\s\\S]*?-----END ${blockType}-----`
    );
    const match = pem.match(regex);
    return match ? match[0] : null;
  }
}
