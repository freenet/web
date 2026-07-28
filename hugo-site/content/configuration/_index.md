---
title: "Data Locations & Resource Limits"
date: 2026-07-27
draft: false
---

Freenet runs a peer in the background. The peer stores contracts, talks to other peers, and uses
some disk, memory, and bandwidth on your machine. This page shows where that data lives and how to
cap what the peer uses.

## Where Freenet stores data

| | macOS | Linux | Windows |
|---|---|---|---|
| Data (contracts, delegates, secrets, database) | `~/Library/Application Support/The-Freenet-Project-Inc.Freenet` | `~/.local/share/freenet` | `%LOCALAPPDATA%\The Freenet Project Inc\Freenet\data` |
| Configuration (`config.toml`) | same directory as data | `~/.config/freenet` | `%LOCALAPPDATA%\The Freenet Project Inc\Freenet\config` |
| Logs | `~/Library/Logs/freenet` | `~/.local/state/freenet` | `%LOCALAPPDATA%\freenet\logs` |

Inside the data directory you'll find `contracts/` (contract code and state), `delegates/`,
`secrets/` (your keys), `db/`, and `wasmtime-cache/` (compiled contract code).

To see how much space it's using on macOS:

```bash
du -sh ~/Library/Application\ Support/The-Freenet-Project-Inc.Freenet
```

Logs look after themselves: files older than 72 hours are deleted at startup, and the log directory
is capped at 512 MB.

## Changing settings

Freenet writes a `config.toml` in the configuration directory above, and reads it on every start.
Edit that file, then restart the peer.

Two rules to avoid a peer that won't start:

- **Edit the generated file, don't write your own.** Freenet needs the whole file, including the
  paths and key locations it wrote itself. A partial `config.toml` fails to parse and the peer
  refuses to start.
- **If it does refuse to start, delete `config.toml` and restart.** Freenet regenerates it with
  defaults. Your data and keys are untouched.

Restart the peer after editing:

- **macOS (Freenet.app):** click the rabbit in the menu bar and choose **Restart**.
- **Linux, or a macOS install via `install.sh`:** `freenet service restart`
- **Windows:** restart Freenet from the tray icon.

The settings below also work as command-line flags (`--max-hosting-storage`) if you run
`freenet network` yourself, and most have an environment variable too. Run `freenet network --help`
for the exact flag and variable names. The background service started by the installer takes no
flags, so `config.toml` is the only way to configure it.

## Resource limits

These are the settings worth knowing about. The names on the left are the `config.toml` keys.

| Setting | Limits | Default |
|---|---|---|
| `max-hosting-storage` | Bytes of contract state the peer keeps for the network. Once past it, contracts are evicted least-useful-first and their disk reclaimed. | 1/8 of system RAM, at least 128 MB and at most 1 GB |
| `hosting-disk-pct` | Fraction of the data disk's capacity used as a second ceiling on the same eviction. | `0.5` |
| `max-hosting-disk` | Hard cap for that disk ceiling. | 32 GB |
| `module-cache-budget-bytes` | Memory for cached compiled contract code. Delegates get a further 1/4 of this. | 1/8 of system RAM, at least 64 MB and at most 4 GB |
| `max_blocking_threads` | Threads used to run contract code, which is the peer's main CPU cost. | 2x CPU cores, at least 4 and at most 32 |
| `max-number-of-connections` | Peer connections accepted. | `200` |
| `min-number-of-connections` | Peer connections the node tries to maintain. Lowering this is the single biggest lever on idle bandwidth. | `25` |
| `total_bandwidth_limit` | Bytes per second across all connections. Unset means no aggregate cap. | unset |
| `bandwidth_limit` | Bytes per second for a single large transfer. | `3000000` (3 MB/s) |

Note the mixed dashes and underscores. Copy the key names exactly as written above.

The disk that contract hosting actually uses is the smaller of the RAM-derived budget
(`max-hosting-storage`) and the disk-derived one (`hosting-disk-pct` of the disk, capped by
`max-hosting-disk`). On a typical laptop the RAM-derived budget binds first, so
`max-hosting-storage` is the setting to change.

### Example: a light-touch peer on a laptop

Add or edit these lines in the generated `config.toml`:

```toml
max-hosting-storage = 268435456   # 256 MB of contract state
module-cache-budget-bytes = 134217728  # 128 MB of compiled-code cache
min-number-of-connections = 10
max-number-of-connections = 40
total_bandwidth_limit = 1000000   # 1 MB/s total
max_blocking_threads = 4
```

All values are in bytes. A peer configured this way still works, it just hosts less for other
people and contributes less to the network.

## Related

- [Uninstalling Freenet](/uninstall/) if you want it gone entirely.
- [Matrix chat](https://matrix.to/#/#freenet-locutus:matrix.org) if something here doesn't match
  what you see.
