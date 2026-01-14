/**
 * Background service worker for Freenet Ghost Keys extension
 */
import { StorageService } from './storage-service';
import { initWasm, signMessage, signAuthRequest, verifySignedMessage } from './crypto-service';
import { MessageType, ExtensionMessage } from '../shared/messages';

const storageService = new StorageService();

// Initialize WASM on startup
initWasm().catch(console.error);

// Handle messages from popup, options, and content scripts
chrome.runtime.onMessage.addListener(
  (message: ExtensionMessage, _sender, sendResponse) => {
    handleMessage(message)
      .then(sendResponse)
      .catch((e) => sendResponse({ error: e.message }));
    return true; // Async response
  }
);

async function handleMessage(message: ExtensionMessage): Promise<any> {
  switch (message.type) {
    case MessageType.IS_UNLOCKED:
      return { unlocked: storageService.isUnlocked() };

    case MessageType.UNLOCK_VAULT:
      const unlocked = await storageService.unlock(message.password);
      return { success: unlocked };

    case MessageType.LOCK_VAULT:
      storageService.lock();
      return { success: true };

    case MessageType.IMPORT_KEY:
      const keyId = await storageService.importKey(message.pemContent, message.label);
      return { success: true, keyId };

    case MessageType.DELETE_KEY:
      await storageService.deleteKey(message.keyId);
      return { success: true };

    case MessageType.GET_KEYS:
      const keys = await storageService.getKeyList();
      return { keys };

    case MessageType.SET_ACTIVE_KEY:
      await storageService.setActiveKey(message.keyId);
      return { success: true };

    case MessageType.EXPORT_KEY:
      const pemContent = await storageService.exportKey(message.keyId);
      return { pemContent };

    case MessageType.SIGN_MESSAGE:
      return await handleSignMessage(message.keyId, message.message);

    case MessageType.SIGN_AUTH_REQUEST:
      return await handleSignAuthRequest(message);

    case MessageType.VERIFY_SIGNATURE:
      const result = await verifySignedMessage(message.signedMessage);
      return result;

    case MessageType.PAGE_AUTH_REQUEST:
      // This comes from content script - needs user confirmation
      return await handlePageAuthRequest(message);

    default:
      throw new Error('Unknown message type');
  }
}

async function handleSignMessage(
  keyId: string,
  message: string
): Promise<{ signedMessage: string }> {
  const key = await storageService.getDecryptedKey(keyId);
  const signedMessage = await signMessage(key.certificate, key.signingKey, message);
  return { signedMessage };
}

async function handleSignAuthRequest(message: {
  origin: string;
  contractAddress?: string;
  challenge: string;
  purpose: string;
}): Promise<{ signedAuth: string; certificate: string }> {
  const key = await storageService.getActiveKey();
  if (!key) {
    throw new Error('No active key');
  }

  return await signAuthRequest(key.certificate, key.signingKey, {
    origin: message.origin,
    contractAddress: message.contractAddress,
    challenge: message.challenge,
    purpose: message.purpose,
  });
}

async function handlePageAuthRequest(message: {
  origin: string;
  contractAddress?: string;
  challenge: string;
  purpose: string;
  requestId: string;
}): Promise<{ signedAuth: string; certificate: string } | { error: string }> {
  // For now, auto-approve if unlocked
  // TODO: Open popup for user confirmation
  if (!storageService.isUnlocked()) {
    return { error: 'Vault is locked' };
  }

  try {
    return await handleSignAuthRequest(message);
  } catch (e) {
    return { error: (e as Error).message };
  }
}

// Log startup
console.log('Freenet Ghost Keys extension loaded');
