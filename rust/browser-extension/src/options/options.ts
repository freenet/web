/**
 * Options page for Freenet Ghost Keys extension
 */
import { MessageType } from '../shared/messages';
import type { KeyMetadata } from '../shared/types';

// Elements
const lockOverlay = document.getElementById('lock-overlay')!;
const passwordInput = document.getElementById('password') as HTMLInputElement;
const passwordError = document.getElementById('password-error')!;
const unlockBtn = document.getElementById('unlock-btn')!;
const lockBtn = document.getElementById('lock-btn')!;
const keysList = document.getElementById('keys-list')!;
const emptyState = document.getElementById('empty-state')!;
const importLabel = document.getElementById('import-label') as HTMLInputElement;
const importPem = document.getElementById('import-pem') as HTMLTextAreaElement;
const importError = document.getElementById('import-error')!;
const importSuccess = document.getElementById('import-success')!;
const importBtn = document.getElementById('import-btn')!;
const exportModal = document.getElementById('export-modal')!;
const exportContent = document.getElementById('export-content') as HTMLTextAreaElement;
const copyExport = document.getElementById('copy-export')!;
const closeExport = document.getElementById('close-export')!;

// State
let keys: KeyMetadata[] = [];

// Initialize
async function init() {
  const response = await chrome.runtime.sendMessage({ type: MessageType.IS_UNLOCKED });

  if (response.unlocked) {
    hideOverlay();
    await loadKeys();
  } else {
    showOverlay();
  }
}

function showOverlay() {
  lockOverlay.classList.remove('hidden');
  passwordInput.focus();
}

function hideOverlay() {
  lockOverlay.classList.add('hidden');
}

async function loadKeys() {
  const response = await chrome.runtime.sendMessage({ type: MessageType.GET_KEYS });
  keys = response.keys || [];
  renderKeys();
}

function renderKeys() {
  if (keys.length === 0) {
    keysList.innerHTML = '';
    emptyState.classList.remove('hidden');
    return;
  }

  emptyState.classList.add('hidden');
  keysList.innerHTML = keys
    .map(
      (key) => `
      <div class="key-card ${key.isActive ? 'active' : ''}" data-id="${key.id}">
        <div class="key-info">
          <div class="key-label">${escapeHtml(key.label)}</div>
          <div class="key-meta">
            Created: ${formatDate(key.createdAt)}
            ${key.isActive ? ' • <span style="color: #4cc9f0;">Active</span>' : ''}
          </div>
        </div>
        <div class="key-actions">
          ${!key.isActive ? `<button class="secondary select-btn" data-id="${key.id}">Set Active</button>` : ''}
          <button class="secondary export-btn" data-id="${key.id}">Export</button>
          <button class="danger delete-btn" data-id="${key.id}">Delete</button>
        </div>
      </div>
    `
    )
    .join('');

  // Add click handlers
  document.querySelectorAll('.select-btn').forEach((btn) => {
    btn.addEventListener('click', async (e) => {
      const keyId = (e.target as HTMLElement).dataset.id!;
      await chrome.runtime.sendMessage({
        type: MessageType.SET_ACTIVE_KEY,
        keyId,
      });
      await loadKeys();
    });
  });

  document.querySelectorAll('.export-btn').forEach((btn) => {
    btn.addEventListener('click', async (e) => {
      const keyId = (e.target as HTMLElement).dataset.id!;
      const response = await chrome.runtime.sendMessage({
        type: MessageType.EXPORT_KEY,
        keyId,
      });
      if (response.pemContent) {
        exportContent.value = response.pemContent;
        exportModal.classList.remove('hidden');
      }
    });
  });

  document.querySelectorAll('.delete-btn').forEach((btn) => {
    btn.addEventListener('click', async (e) => {
      const keyId = (e.target as HTMLElement).dataset.id!;
      const key = keys.find((k) => k.id === keyId);
      if (confirm(`Delete key "${key?.label}"? This cannot be undone.`)) {
        await chrome.runtime.sendMessage({
          type: MessageType.DELETE_KEY,
          keyId,
        });
        await loadKeys();
      }
    });
  });
}

function formatDate(timestamp: number): string {
  return new Date(timestamp).toLocaleDateString();
}

function escapeHtml(str: string): string {
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

// Event handlers
unlockBtn.addEventListener('click', async () => {
  const password = passwordInput.value;
  if (!password) {
    showError(passwordError, 'Please enter a password');
    return;
  }

  const response = await chrome.runtime.sendMessage({
    type: MessageType.UNLOCK_VAULT,
    password,
  });

  if (response.success) {
    passwordInput.value = '';
    passwordError.classList.add('hidden');
    hideOverlay();
    await loadKeys();
  } else {
    showError(passwordError, 'Incorrect password');
  }
});

passwordInput.addEventListener('keypress', (e) => {
  if (e.key === 'Enter') {
    unlockBtn.click();
  }
});

lockBtn.addEventListener('click', async () => {
  await chrome.runtime.sendMessage({ type: MessageType.LOCK_VAULT });
  showOverlay();
});

importBtn.addEventListener('click', async () => {
  const label = importLabel.value.trim();
  const pem = importPem.value.trim();

  importError.classList.add('hidden');
  importSuccess.classList.add('hidden');

  if (!label) {
    showError(importError, 'Please enter a label');
    return;
  }

  if (!pem) {
    showError(importError, 'Please paste your PEM content');
    return;
  }

  if (!pem.includes('-----BEGIN GHOSTKEY_CERTIFICATE_V1-----')) {
    showError(importError, 'Invalid PEM: missing ghost key certificate');
    return;
  }

  if (!pem.includes('-----BEGIN SIGNING_KEY_V1-----')) {
    showError(importError, 'Invalid PEM: missing signing key');
    return;
  }

  try {
    const response = await chrome.runtime.sendMessage({
      type: MessageType.IMPORT_KEY,
      pemContent: pem,
      label,
    });

    if (response.error) {
      showError(importError, response.error);
    } else {
      importLabel.value = '';
      importPem.value = '';
      importSuccess.textContent = 'Key imported successfully!';
      importSuccess.classList.remove('hidden');
      await loadKeys();
    }
  } catch (e) {
    showError(importError, (e as Error).message);
  }
});

copyExport.addEventListener('click', async () => {
  await navigator.clipboard.writeText(exportContent.value);
  copyExport.textContent = 'Copied!';
  setTimeout(() => {
    copyExport.textContent = 'Copy to Clipboard';
  }, 2000);
});

closeExport.addEventListener('click', () => {
  exportModal.classList.add('hidden');
});

function showError(element: HTMLElement, message: string) {
  element.textContent = message;
  element.classList.remove('hidden');
}

// Start
init();
