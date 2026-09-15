---
title: "Freenet on Nix"
date: 2026-09-15
draft: false
---

Freenet runs on Nix and NixOS as a supported deployment. A peer installed this way keeps itself up to date exactly like every other install method, using the same signed release channel.

## Install

```bash
nix run github:freenet/freenet-core/vX.Y.Z
```

Use a real release tag rather than the default branch. The default branch can carry a version number that has not been published yet, and a peer that starts out ahead of every published release will not update until one overtakes it. Any tag from v0.2.136 onward works.

Arguments are passed through:

```bash
nix run github:freenet/freenet-core/vX.Y.Z -- --config-dir /srv/freenet
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

The honest cost: the running binary drifts away from the store path it came from, so `nix run` reports the version it seeded rather than the version now running. That is deliberate. Pinning the binary to the store would mean pinning the peer to whatever release your flake happened to name, which is the outcome this design exists to avoid.

## NixOS

```nix
systemd.services.freenet-node = {
  wantedBy = [ "multi-user.target" ];
  serviceConfig = {
    ExecStart = "${pkgs.freenet-node}/bin/freenet-node";
    User = "freenet";
    StateDirectory = "freenet";
    Restart = "always";
    RestartSec = 30;
  };
};

users.users.freenet = {
  isSystemUser = true;
  group = "freenet";
  home = "/var/lib/freenet";
  createHome = true;
};
users.groups.freenet = { };
```

The `home` line is load bearing and easy to leave out. `StateDirectory` is not the only directory that has to be writable. The node keeps its auto-update state, meaning the crash-probation marker, the rollback snapshot and the known-bad version pin, under the service user's home directory. A NixOS user declared without `home` gets `/var/empty`, which is not writable, and the peer then runs with crash-loop rollback silently switched off. It will still update. It just loses the safety net that recovers it from a bad release.

Keep `Restart = "always"`. If another process is already holding the node's port, the supervisor stands down and lets systemd retry rather than fighting it, and `always` is what brings the peer back once the port frees.

## Further reading

Full detail, including the update contract, the exit codes the supervisor honours and what it deliberately does not do, is in [`docs/nix.md`](https://github.com/freenet/freenet-core/blob/main/docs/nix.md) in the freenet-core repository.
