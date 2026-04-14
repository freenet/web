---
title: "Remote Access to a Node"
date: 2026-04-14
draft: false
---

By default a Freenet node's local HTTP/WebSocket API (port `7509`) only accepts
connections from the **same machine** and from **private-LAN addresses**
(`127.0.0.1`, RFC 1918 ranges, IPv6 link-local, IPv6 ULA). Anything that can
reach that port can use your node's full client API (read and write contract
state, call delegates, and issue requests on your behalf), so the default is
intentionally restrictive.

This page explains how to access your node from a device that is *not* on your
LAN (for example, from your phone while you're out) without giving up that
security posture.

> ⚠️ **The 7509 API is fully privileged.** Treat it like SSH: only expose it
> over channels you fully control. Never open it to the public internet, and
> never extend the allowlist to shared address space (CGNAT ranges such as
> `100.64.0.0/10` are shared between subscribers of some ISPs and are only
> safe on a private overlay you control).

## Option 1 (safest): SSH tunnel

No configuration changes required. On the remote device:

```bash
ssh -L 7509:127.0.0.1:7509 you@your-node-host
```

Then point your browser or client at `http://127.0.0.1:7509/`. The traffic is
authenticated and encrypted by SSH, and the node itself never exposes 7509 to
anything other than loopback.

## Option 2: Tailscale (or another private overlay)

If you use [Tailscale](https://tailscale.com/), [WireGuard](https://www.wireguard.com/),
Nebula, or a similar overlay, your devices get private IPs on a network only
you control. You can grant the node's API access to that overlay with two
steps:

1. **Bind the API to the overlay interface** (or to `0.0.0.0` / `::`) so the
   socket is reachable from the overlay at all:

   ```toml
   # ~/.config/freenet/config.toml
   [ws-api]
   ws-api-address = "0.0.0.0"
   ws-api-port = 7509
   ```

2. **Extend the source-IP allowlist** to cover the overlay's address range.
   For Tailscale, that's the CGNAT range `100.64.0.0/10`, or, ideally, a
   narrower CIDR matching just your tailnet's assigned subnet:

   ```toml
   [ws-api]
   allowed-source-cidrs = ["100.64.0.0/10"]
   # Or, stricter: only addresses in your assigned tailnet subnet:
   # allowed-source-cidrs = ["100.64.1.0/24"]
   ```

   You can also pass this on the command line or via environment variable:

   ```bash
   freenet --allowed-source-cidrs 100.64.0.0/10
   # or
   ALLOWED_SOURCE_CIDRS=100.64.0.0/10 freenet
   ```

   Multiple ranges can be supplied by repeating the flag or using a
   comma-separated list. IPv6 CIDRs (e.g. Tailscale's `fd7a:115c:a1e0::/48`)
   are supported.

With both set, the node will accept API requests from any device on your
tailnet but continue to reject everything else.

### What the allowlist does *not* do

`allowed-source-cidrs` only **extends** the built-in private-IP check. It is
never a substitute for one of:

- binding the API to a private interface you control, or
- running the API behind a VPN, SSH tunnel, or authenticated reverse proxy.

In particular, **do not add a public CIDR to this list**. Doing so publishes
your node's full client API to anyone who can route packets to that address.

## Why the default is strict

Earlier versions of the node rejected every non-private source IP with the
message *"Only local network connections are allowed"*. Keeping that default
unchanged means the security posture for users who don't configure anything is
identical to what it has always been: loopback and LAN only. Opting in to a
wider allowlist is an explicit choice and requires naming the exact ranges you
trust.
