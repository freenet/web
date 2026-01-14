/**
 * Popup UI for Freenet Ghost Keys extension
 */
import { MessageType } from '../shared/messages';
import type { KeyMetadata } from '../shared/types';

// Elements
const lockScreen = document.getElementById('lock-screen')!;
const mainScreen = document.getElementById('main-screen')!;
const passwordInput = document.getElementById('password') as HTMLInputElement;
const passwordError = document.getElementById('password-error')!;
const unlockBtn = document.getElementById('unlock-btn')!;
const lockBtn = document.getElementById('lock-btn')!;
const keysList = document.getElementById('keys-list')!;
const emptyState = document.getElementById('empty-state')!;
const openOptions = document.getElementById('open-options')!;

// State
let keys: KeyMetadata[] = [];

// Initialize
async function init() {
  const response = await chrome.runtime.sendMessage({ type: MessageType.IS_UNLOCKED });

  if (response.unlocked) {
    showMainScreen();
  } else {
    showLockScreen();
  }
}

function showLockScreen() {
  lockScreen.classList.add('active');
  mainScreen.classList.remove('active');
  passwordInput.focus();
}

async function showMainScreen() {
  lockScreen.classList.remove('active');
  mainScreen.classList.add('active');
  await loadKeys();
}

async function loadKeys() {
  const response = await chrome.runtime.sendMessage({ type: MessageType.GET_KEYS });
  keys = response.keys || [];
  renderKeys();
}

function renderKeys() {
  if (keys.length === 0) {
    keysList.innerHTML = '';
    emptyState.style.display = 'block';
    return;
  }

  emptyState.style.display = 'none';
  keysList.innerHTML = keys
    .map(
      (key) => `
      <div class="key-item ${key.isActive ? 'active' : ''}" data-id="${key.id}">
        <div>
          <div class="key-label">${escapeHtml(key.label)}</div>
          <div class="key-date">${formatDate(key.createdAt)}</div>
        </div>
        <div class="actions">
          ${
            key.isActive
              ? '<span style="color: #4cc9f0; font-size: 12px;">Active</span>'
              : `<button class="secondary select-btn" data-id="${key.id}">Select</button>`
          }
        </div>
      </div>
    `
    )
    .join('');

  // Add click handlers for select buttons
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
    showError('Please enter a password');
    return;
  }

  const response = await chrome.runtime.sendMessage({
    type: MessageType.UNLOCK_VAULT,
    password,
  });

  if (response.success) {
    passwordInput.value = '';
    passwordError.style.display = 'none';
    showMainScreen();
  } else {
    showError('Incorrect password');
  }
});

passwordInput.addEventListener('keypress', (e) => {
  if (e.key === 'Enter') {
    unlockBtn.click();
  }
});

lockBtn.addEventListener('click', async () => {
  await chrome.runtime.sendMessage({ type: MessageType.LOCK_VAULT });
  showLockScreen();
});

openOptions.addEventListener('click', (e) => {
  e.preventDefault();
  chrome.runtime.openOptionsPage();
});

function showError(message: string) {
  passwordError.textContent = message;
  passwordError.style.display = 'block';
}

// Start
init();
