---
title: "Publish a Website"
date: 2026-04-11
draft: false
weight: 0
---

Freenet can host static websites -- HTML, CSS, JavaScript, images -- with no server required. Your
site is distributed across the peer-to-peer network, served through any Freenet gateway, and can
only be updated by you.

No programming is required. If you have a folder of static files with an `index.html`, you can
publish it to Freenet in three commands.

---

## How It Works

Your website files are compressed into an archive, signed with your private key, and stored as a
Freenet contract. The contract enforces two rules:

1. **Authentication**: only someone with your signing key can publish or update the site
2. **Versioning**: updates must have a higher version number (no rollbacks)

The contract's key (a hash of the WASM code and your public key) becomes your website's permanent
address. Anyone running a Freenet node can access it through their local gateway.

---

## Prerequisites

Install Freenet using the [quickstart guide](/quickstart/), which includes the `fdev` tool and a
running Freenet node.

---

## 1. Generate a Signing Keypair

Choose a name for your website and run:

```bash
fdev website init my-blog
```

This creates an Ed25519 keypair at `~/.config/freenet/website-keys/my-blog.toml` and prints your
website's contract key and URL:

```
Keypair 'my-blog' generated and saved to: /home/you/.config/freenet/website-keys/my-blog.toml

Your website contract key: 3ZZ98ojKWUJsixNyJsgRwkBZhLxN4CV2Z5AT8dVWJh48
Website URL: http://127.0.0.1:7509/v1/contract/web/3ZZ98ojKWUJsixNyJsgRwkBZhLxN4CV2Z5AT8dVWJh48/

To publish:  fdev website publish ./my-site/ --key my-blog
To update:   fdev website update ./my-site/ --key my-blog

IMPORTANT: Back up your key file! Losing it means you can never update your website.
```

The contract key is derived from your public key and the contract code. It is your website's
permanent address -- it will not change when you update the site content.

> **Back up your key file.** The signing key is the only thing that authorizes updates to your
> website. If you lose it, the site becomes permanently read-only. There is no recovery mechanism.
> Copy the `.toml` file to a password manager (1Password, Bitwarden, etc.), an encrypted USB drive,
> or any other secure backup you trust.

---

## 2. Publish Your Website

Point `fdev` at a directory containing your website files. The directory must contain an
`index.html` at its root.

```bash
fdev website publish ./my-site/ --key my-blog
```

This compresses the directory, signs it, and publishes it to your local Freenet node. The node then
distributes it across the network.

```
Compressed ./my-site/ -> 48231 bytes (12 files)
Publishing website as contract 3ZZ98ojKWUJsixNyJsgRwkBZhLxN4CV2Z5AT8dVWJh48 (version 29523847)
Website published successfully!
URL: http://127.0.0.1:7509/v1/contract/web/3ZZ98ojKWUJsixNyJsgRwkBZhLxN4CV2Z5AT8dVWJh48/
```

Visit the URL in your browser to see the site served from Freenet.

---

## 3. Update Your Website

Edit your files, then run:

```bash
fdev website update ./my-site/ --key my-blog
```

The version number increments automatically. The contract rejects any update that doesn't have a
higher version than the current one, so only forward progress is possible.

---

## Static Site Generators

Any static site generator works -- Hugo, Jekyll, Eleventy, Astro, mkdocs, or plain HTML. Just point
`fdev` at the build output directory:

```bash
# Hugo
hugo --minify
fdev website publish ./public/ --key my-blog

# Eleventy
npx @11ty/eleventy
fdev website publish ./_site/ --key my-blog

# Astro
npm run build
fdev website publish ./dist/ --key my-blog

# Plain HTML
fdev website publish ./my-site/ --key my-blog
```

### Considerations for Freenet-hosted sites

Sites served through a Freenet gateway run inside a sandboxed iframe at a path like
`/v1/contract/web/<contract-key>/`. Keep these in mind:

- **Set `baseURL` to your contract path** for static site generators. For Hugo:
  `hugo --baseURL "/v1/contract/web/<contract-key>/"`. This ensures CSS, JS, and image paths
  resolve correctly inside the gateway iframe.
- **Use your generator's URL functions** (Hugo's `relURL`, Jekyll's `relative_url`) instead of
  hardcoding absolute paths like `/style.css`. Hardcoded paths need post-processing to include
  the contract base path.
- **Multi-page navigation requires JavaScript** -- the gateway's iframe sandbox blocks regular
  link clicks from navigating between pages. Add a click interceptor that uses
  `window.location.href` for internal links if your site has multiple pages. Single-page apps
  (SPAs) work without this since they handle routing in JavaScript.
- **No server-side logic** -- no PHP, no server-side rendering, no API routes
- **No external API calls that require CORS** -- the gateway iframe sandbox blocks cross-origin
  requests
- **External resources** (CDN fonts, external images) may not load due to sandbox restrictions.
  Bundle fonts and images locally.
- **Large sites work fine** -- the archive is compressed with xz; the contract supports up to 100MB

---

## Using a Custom Contract

If you have an existing website contract (e.g., River's web container) and want to keep its contract
key, use the `--contract-wasm` flag:

```bash
fdev website publish ./my-site/ --key my-blog --contract-wasm ./my-contract.wasm
```

This uses your custom WASM for the contract while still handling compression, signing, and
publishing automatically.

---

## Key Management

### How keys are stored

Each website gets its own key file at `~/.config/freenet/website-keys/<name>.toml`. The name you
choose during `fdev website init` identifies the key for all subsequent publish/update commands.

To see all your website keys and their contract addresses:

```bash
fdev website list
```

### Multiple websites

Each name produces a different keypair and contract key:

```bash
fdev website init blog
fdev website init docs

fdev website publish ./blog/public/ --key blog
fdev website publish ./docs/site/   --key docs
```

### Backing up your keys

Your signing key is the only thing that authorizes updates to your website. If you lose it, the
site becomes permanently read-only with no recovery mechanism.

The key file is a small `.toml` text file. Back it up wherever you keep important credentials:

- **Password manager** (1Password, Bitwarden, KeePassXC) -- store the file contents as a secure note
- **Encrypted backup** -- copy the `~/.config/freenet/website-keys/` directory to an encrypted drive
- **Version control** (private repo) -- if you trust your private repo's access controls

Do not commit key files to public repositories.

---

## How It Works (Technical Details)

The website container contract is a standard Freenet contract with a specific state format:

```
[metadata_length: u64 BE][metadata: CBOR][web_length: u64 BE][web: tar.xz archive]
```

The metadata contains a version number and an Ed25519 signature over `version_bytes || archive_bytes`.
The contract parameters are the 32-byte Ed25519 verifying key.

On `validate_state`, the contract verifies the signature. On `update_state`, it additionally checks
that the new version is strictly greater than the current version. The state synchronization methods
(`summarize_state`, `get_state_delta`) use the version number for efficient peer sync.

The contract source code is in the
[freenet-website-contract](https://github.com/freenet/freenet-core/tree/main/crates/website-contract)
crate.
