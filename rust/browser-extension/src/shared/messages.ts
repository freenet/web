/**
 * Message types for communication between extension components
 */
export enum MessageType {
  // Vault operations
  UNLOCK_VAULT = 'UNLOCK_VAULT',
  UNLOCK_VAULT_PASSKEY = 'UNLOCK_VAULT_PASSKEY',
  LOCK_VAULT = 'LOCK_VAULT',
  IS_UNLOCKED = 'IS_UNLOCKED',

  // Key management
  IMPORT_KEY = 'IMPORT_KEY',
  DELETE_KEY = 'DELETE_KEY',
  GET_KEYS = 'GET_KEYS',
  SET_ACTIVE_KEY = 'SET_ACTIVE_KEY',
  EXPORT_KEY = 'EXPORT_KEY',

  // Signing operations
  SIGN_MESSAGE = 'SIGN_MESSAGE',
  SIGN_AUTH_REQUEST = 'SIGN_AUTH_REQUEST',
  VERIFY_SIGNATURE = 'VERIFY_SIGNATURE',

  // Website requests (from content script)
  PAGE_AUTH_REQUEST = 'PAGE_AUTH_REQUEST',
}

export interface UnlockVaultMessage {
  type: MessageType.UNLOCK_VAULT;
  password: string;
}

export interface UnlockVaultPasskeyMessage {
  type: MessageType.UNLOCK_VAULT_PASSKEY;
}

export interface LockVaultMessage {
  type: MessageType.LOCK_VAULT;
}

export interface IsUnlockedMessage {
  type: MessageType.IS_UNLOCKED;
}

export interface ImportKeyMessage {
  type: MessageType.IMPORT_KEY;
  pemContent: string;
  label: string;
}

export interface DeleteKeyMessage {
  type: MessageType.DELETE_KEY;
  keyId: string;
}

export interface GetKeysMessage {
  type: MessageType.GET_KEYS;
}

export interface SetActiveKeyMessage {
  type: MessageType.SET_ACTIVE_KEY;
  keyId: string;
}

export interface ExportKeyMessage {
  type: MessageType.EXPORT_KEY;
  keyId: string;
}

export interface SignMessageMessage {
  type: MessageType.SIGN_MESSAGE;
  keyId: string;
  message: string;
}

export interface SignAuthRequestMessage {
  type: MessageType.SIGN_AUTH_REQUEST;
  origin: string;
  contractAddress?: string;
  challenge: string;
  purpose: string;
}

export interface VerifySignatureMessage {
  type: MessageType.VERIFY_SIGNATURE;
  signedMessage: string;
}

export interface PageAuthRequestMessage {
  type: MessageType.PAGE_AUTH_REQUEST;
  origin: string;
  contractAddress?: string;
  challenge: string;
  purpose: string;
  requestId: string;
}

export type ExtensionMessage =
  | UnlockVaultMessage
  | UnlockVaultPasskeyMessage
  | LockVaultMessage
  | IsUnlockedMessage
  | ImportKeyMessage
  | DeleteKeyMessage
  | GetKeysMessage
  | SetActiveKeyMessage
  | ExportKeyMessage
  | SignMessageMessage
  | SignAuthRequestMessage
  | VerifySignatureMessage
  | PageAuthRequestMessage;
