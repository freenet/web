# Freenet Ghost Keys Browser Extension

Chrome extension for managing Freenet ghost keys - anonymous, verifiable identities backed by donations.

## Features

- Password-protected encrypted vault for key storage
- Import/export ghost key certificates (PEM format)
- Sign messages with your ghost key
- Page authentication via content script injection

## Building

### Prerequisites

- Node.js 18+
- The `gkwasm` module built (see `../gkwasm/`)

### Build Steps

```bash
# Install dependencies
npm install

# Copy WASM files from gkwasm build
mkdir -p wasm
cp ../gkwasm/pkg/gkwasm_bg.wasm wasm/
cp ../gkwasm/pkg/gkwasm.js wasm/

# Build the extension
npm run build
```

### Loading in Chrome

1. Open `chrome://extensions/`
2. Enable "Developer mode" (top right)
3. Click "Load unpacked"
4. Select this directory (`rust/browser-extension/`)

## Usage

1. **First run**: Set a password to create your encrypted vault
2. **Import keys**: Use the options page to import ghost key PEM files from your donation
3. **Sign messages**: Select an active key and use it to sign messages on supported sites

## Development

```bash
# Watch mode for development
npm run watch
```

The extension uses:
- TypeScript for type safety
- Webpack for bundling
- Web Crypto API for vault encryption (AES-GCM)
- WASM (gkwasm) for ghost key cryptographic operations

## Architecture

```
src/
├── background/       # Service worker (vault, signing)
│   ├── index.ts      # Message handler
│   ├── storage-service.ts  # Encrypted key storage
│   └── crypto-service.ts   # WASM wrapper
├── popup/            # Browser action popup
├── options/          # Full key management page
├── content/          # Page injection for auth
└── shared/           # Types and messages
```

## Icons

The extension needs icons at `static/icons/`:
- `icon-16.png` (16x16)
- `icon-48.png` (48x48)
- `icon-128.png` (128x128)
