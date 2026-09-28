---
title: "Freenet on Nix"
date: 2026-09-15
draft: false
---

Freenet runs on Nix and NixOS as a supported deployment. A peer installed this way keeps itself up to date exactly like every other install method, using the same signed release channel.

## Install

```bash
nix run github:freenet/freenet-core/v0.2.139
```

Use a release tag rather than the default branch. The default branch can carry a version number that has not been published yet, and a peer that starts out ahead of every published release will not update until one overtakes it. Any tag from v0.2.136 onward works, and it does not need to be the newest one: the peer updates itself to the current release on its first run.

Arguments are passed through:

```bash
nix run github:freenet/freenet-core/v0.2.139 -- --config-dir /srv/freenet
```

{{< alert type="warning" >}} **Your peer updates itself.** Freenet is under active development and releases land often, sometimes several times a day. Older versions stop working as the network moves on, so a peer that stops updating stops being useful to the network. Auto-updates are on by default and should be left on. {{< /alert >}}

## Two outputs, and only one of them runs a peer

| Output | Runs a peer? |
|---|---|
| `packages.freenet-node`, which is also `packages.default` | **Yes.** The supervised, self-updating node. This is the supported way to run a peer. |
| `packages.freenet` | **No.** The bare compiler output, for `nix develop`, CI and packaging. |

`packages.freenet` is a build artifact, not a deployment. It has no supervisor, so nothing applies an update or restarts the node afterwards. A peer started that way falls behind and eventually stops working. If you want to run a peer, run `packages.freenet-node`.

## Why the binary does not live in the Nix store

A Freenet node updates by replacing its own executable, after checking the new release against a signing key built into the binary. The Nix store is read only, so that cannot happen there.

Instead, Nix seeds the binary once into a writable state directory, and the node maintains it from that point on. You get the normal Nix build and the normal update path, including signature verification, crash-loop rollback and known-bad version pinning.

The honest cost: the running binary drifts away from the store path it came from, so the store path names the version it seeded rather than the version now running. That is deliberate. Pinning the binary to the store would mean pinning the peer to whatever release your flake happened to name, which is the outcome this design exists to avoid.

## NixOS

Add freenet-core as a flake input and hand its `freenet-node` package to your configuration:

```nix
{
  inputs.freenet.url = "github:freenet/freenet-core/v0.2.139";

  outputs = { nixpkgs, freenet, ... }: {
    nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      specialArgs.freenet-node = freenet.packages.x86_64-linux.freenet-node;
      modules = [ ./configuration.nix ];  # which takes { freenet-node, ... }
    };
  };
}
```

Use the package rather than the flake's overlay. The package is built with the Rust toolchain Freenet pins, while the overlay builds with whatever compiler your nixpkgs has, which on a stable channel can be too old.

The systemd unit to put in `configuration.nix` is in the [NixOS section of `docs/nix.md`](https://github.com/freenet/freenet-core/blob/main/docs/nix.md#running-it-under-systemd-on-nixos). Copy it whole rather than writing your own. Several of its settings look optional and are not. In particular, the service user needs a writable home directory, because that is where the node keeps its crash-loop rollback state. Without it the peer still updates, but a bad release has nothing to roll it back.

## Further reading

Full detail, including the update contract, the exit codes the supervisor honours and what it deliberately does not do, is in [`docs/nix.md`](https://github.com/freenet/freenet-core/blob/main/docs/nix.md) in the freenet-core repository.
