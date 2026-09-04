---
title: "Run Freenet in Docker"
date: 2025-01-01
draft: false
---

Freenet publishes an official container image for every release. It is built for
`linux/amd64` and `linux/arm64`, so it runs on an ordinary server, a NAS, or a
Raspberry Pi.

This page is for running a peer on a machine you administer. If you just want to
use Freenet on your own computer, the [normal install](/quickstart/) is simpler
and does more for you.

## Start a node

```bash
docker run -d --name freenet-node --network host \
  -v freenet-data:/data --restart unless-stopped \
  ghcr.io/freenet/freenet-core:latest
```

Then open <http://127.0.0.1:7509/> to reach the node's dashboard and any Freenet
app it is serving.

Or, with a `compose.yml`:

```yaml
services:
  freenet-node:
    image: ghcr.io/freenet/freenet-core:latest
    network_mode: host
    volumes:
      - freenet-data:/data
    restart: unless-stopped
    stop_grace_period: 45s

volumes:
  freenet-data:
```

```bash
docker compose up -d
docker compose logs -f
```

## It keeps itself up to date

**You do not need Watchtower, a cron job, or a habit of running
`docker compose pull`.** A container started once stays current on its own.

This matters more than it does for most software. Freenet ships releases
frequently, sometimes several times a day, and a peer that falls too far behind
is refused by the rest of the network rather than merely missing features. So the
container applies updates itself, the same way the desktop install does.

Pulling a newer image is still worth doing occasionally, so a fresh container
starts from a recent version instead of updating on first boot:

```bash
docker compose pull && docker compose up -d
```

Your node's data lives in the `freenet-data` volume and survives that, along with
any update the node has already applied to itself.

To see which version is actually running:

```bash
docker exec freenet-node freenet --version
```

## Why `--network host`

Two things break under Docker's default bridge network, and neither is obvious
from the outside.

**The dashboard becomes unreachable.** Freenet's local API binds to loopback,
which under bridge networking is the *container's* loopback rather than your
machine's. Nothing on the host can reach it, and publishing the port does not
help, because the API is not listening on an address that port forwards to.

**Peer-to-peer connectivity degrades.** Freenet peers talk over UDP and rely on
hole punching. Bridge networking rewrites the source port of outgoing packets, so
it no longer matches the port other peers were told to use.

Host networking avoids both. It needs Linux; Docker Desktop on macOS and Windows
does not support it in the same way.

### If you cannot use host networking

This works, with the caveat that the node contributes capacity to the network but
cannot serve apps to your browser:

```yaml
services:
  freenet-node:
    image: ghcr.io/freenet/freenet-core:latest
    ports:
      - "31337:31337/udp"
    volumes:
      - freenet-data:/data
    restart: unless-stopped
    stop_grace_period: 45s

volumes:
  freenet-data:
```

## Ports

| Port | Protocol | Purpose |
|------|----------|---------|
| 31337 | UDP | Peer connections. Other peers reach you here. |
| 7509 | TCP | Dashboard and local API. Loopback only. |

Port 7509 is deliberately not exposed to your network. It can read and modify
contract state, identities and key material, so treat it like a database socket
rather than a web page. If you need to reach it from another machine, put an
authenticating reverse proxy in front of it.

## Checking on it

```bash
docker compose logs -f                                    # what the node is doing
docker inspect --format '{{.State.Health.Status}}' freenet-node
docker exec freenet-node ls /data/logs                    # rotating log files
```

The health status reports whether the node is up and serving. It does not tell
you how well connected it is.

## Full reference

The [container README](https://github.com/freenet/freenet-core/blob/main/docker/freenet-node/README.md)
covers the remaining details: every environment variable, running under a
different user, how the image is built and verified, and how the self-update
supervisor works.
