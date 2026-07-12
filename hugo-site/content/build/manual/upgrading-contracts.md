---
title: "Upgrading Contracts and Delegates"
date: 2026-07-10
draft: false
---

Upgrading a Freenet contract or delegate is **low-risk and mechanical when you design for it.** A
routine `freenet-stdlib` bump, a dependency update, or a code change is a one-line diff plus a
republish; existing users keep their data, and their invites and share links keep working. River
ships upgrades this way, and its 0.6→0.8 re-key on the live network went through with no room lost
and no invite reissued. You do **not** recreate instances, rotate keys, or warn users that their
links are dead. Recreation is only for one thing: deliberately changing the _owner_ identity (a
compromised key, or a genuinely new instance), which is not what a routine contract or stdlib bump
is.

That "when you design for it" is one design decision, made **before the first release**: anchor your
app's identity on a stable user/owner key, never on the contract key. Freenet derives a contract's
key from its WASM, so a rebuild that changes the bytes changes the key. If your invites and
addresses point at the _owner key_ instead, that key change is transparent: state migrates itself on
the next load, and every reference that embeds the owner identity re-derives the new contract key on
its own. This page shows how to get that property, the one operational step an upgrade requires
(register the outgoing code hash, then republish), and what goes wrong if you skip the design
decision.

If you have not yet read [Contracts](/build/manual/components/contracts/) and
[Delegates](/build/manual/components/delegates/), start there. This page assumes you know what
contract state and delegate secrets are.

## What a re-key costs you: almost nothing, if you designed for it

When the WASM changes and the contract key moves, a dApp that anchored identity on the owner key
sees the move as a no-op from the user's side. Concretely, on the next load:

- **State migrates itself.** The updated client re-derives the new contract key from its bundled
  WASM, reads the previous generation's state from the old key (found via a committed registry of
  past code hashes — the "backward probe" in Step 3), folds it forward, and re-PUTs it under the new
  key. This is permissionless: because the new contract's `validate_state` re-checks every byte, any
  client can carry the state forward, not just the original author. Delegate secrets migrate locally
  the same way.
- **References that embed the owner key survive.** Invites, share links, room and membership
  references, and any external service keyed on the owner identity keep working across the re-key,
  because they carry the _owner key_, not the contract key — the client re-derives the new contract
  key (`blake3(new_code_hash ‖ owner_key)`) from the unchanged owner identity. They do **not** die
  on an upgrade. (River's invitation, for instance, embeds the room owner's verifying key, so a link
  minted under 0.6 resolves correctly under 0.8.)
- **The only required step is registration.** Before you ship the WASM change, add the _outgoing_
  code hash to your legacy-hash registry (Step 3), then republish. No recreation, no key rotation,
  no "your links are dead" notice.

These are consequences of **designing for them**, not automatic properties of every Freenet app. You
get them when you build in three things: identity derived from a stable key (not the contract key),
state that is self-authorizing and backward-compatible (or transformed with a written carry-forward,
which the [`freenet-migrate`](https://github.com/freenet/freenet-migrate) crate packages), and a
legacy-hash registry the client can probe. The honest caveats: migration is **per-client, on next
load** (not an instant network-wide flip), and a **fresh device has no local state to migrate** — it
just derives the current key and starts clean. The rest of this page is how to build those three
things in, and the reproducible-build discipline that keeps the key from moving by _accident_.

## Under the hood: a new WASM binary is a new key

The mechanical upgrade above works because of how keys are derived, and the same derivation is what
strands data in an app that _didn't_ design for it. A contract's identity is derived entirely from
its code and its parameters:

```text
code_hash    = blake3(wasm)
contract key = blake3(code_hash ‖ params)      // = blake3(blake3(wasm) ‖ params)
delegate key = blake3(code_hash ‖ params)      // delegates derive their key the same way
```

`params` is the fixed parameter bytes you chose at creation time; `wasm` is the exact compiled
binary. This is the derivation Freenet's own
[`ContractInstanceId`](https://docs.rs/freenet-stdlib/latest/freenet_stdlib/prelude/struct.ContractInstanceId.html)
uses, so there is no way around it: **any change to the compiled WASM produces a different key.**

The trap is that "a change to the compiled WASM" is much broader than "I changed my contract's
logic". It includes:

- editing your contract or delegate code, of course;
- bumping a dependency, or a bump in a _transitive_ dependency you never named;
- building with a newer Rust compiler, or a different `wasm-opt`.

So the key does not only move when you intend to ship a v2. It can move on an ordinary rebuild you
thought was identical. That is fine when you have designed for it — the migration in the previous
section carries the state forward and the owner-key references still resolve. It is only a problem
in an app that skipped that design: one that pinned identity to the contract key, or shipped no
legacy-hash registry for the client to probe. There, the old state and the old delegate's secrets
are still on the network, but under a key nothing points at anymore, and from the user's side the
data silently disappears: rooms vanish, an inbox comes up empty, saved sites are gone. This has
bitten real Freenet apps that hadn't yet built the machinery in (River early on; Delta lost per-site
data in April 2026), which is why the design decision matters and gets its own page.

There are two separate jobs here, and you need both:

1. **Make your builds reproducible**, so the key changes _only_ when you decide it should.
2. **Have a migration plan**, so that when the key _does_ change, existing users' data (and their
   invites and links) comes with it.

## Step 1: Make builds reproducible

Do this first. If your builds are not reproducible, you cannot even tell an intentional upgrade from
an accidental re-key, and every migration you write rests on sand.

The baseline is the same discipline as any reproducible Rust build, applied to the crate that
compiles to the contract or delegate WASM:

- **Commit the contract's `Cargo.lock`.** Do not rely on the workspace lockfile — see the
  feature-unification caveat below.
- **Pin the toolchain** with a committed `rust-toolchain.toml`. The rustc version affects the
  emitted WASM bytes.
- **Build `--locked`** in CI and for releases, so a build fails rather than silently resolving a new
  dependency version.
- **Pin the dependencies that affect WASM output** with exact `=x.y.z` versions in the contract's
  manifest, so a `cargo update` at the workspace root cannot rotate your key.

That gets you most of the way, but a committed lockfile plus a pinned toolchain still miss several
things that change the bytes:

- **`wasm-opt` / Binaryen.** Build pipelines (including the Dioxus CLI) post-process the WASM with
  `wasm-opt`. Different Binaryen versions emit different bytes, and the version is not captured by
  `Cargo.lock`. Pin or record the `wasm-opt` version alongside the toolchain.
- **The UI toolchain.** For a web app, the Dioxus CLI (`dx`) version and its bundling step affect
  the packaged archive. Note also the caveat below on webapp/facade contract IDs.
- **Absolute build paths.** rustc can embed the absolute path of your checkout into the binary (for
  example in panic messages and debug info), so the _same source built in two different directories_
  produces two different WASMs. Build with `-C trim-paths` (or `--remap-path-prefix`) to strip them.
- **Building the contract alone vs. co-built with the workspace.** This is the subtle one. If your
  contract is a member of the same Cargo workspace as your UI crate, Cargo's feature unification can
  enable features on shared dependencies that the contract would not enable on its own, and a
  routine UI-side dependency bump then shifts the contract's WASM — and its key — with no change to
  the contract's own code. The fix is to **exclude** every stateful contract and delegate from the
  workspace, give each its own `Cargo.lock`, pin its dependencies with `=x.y.z`, and pin its
  `CARGO_TARGET_DIR` so it never shares a `target/` with the workspace build. The
  [dApp build-system reference](https://github.com/freenet/freenet-agent-skills/blob/main/skills/dapp-builder/references/build-system.md)
  has the exact `Cargo.toml` and `Makefile.toml` stanzas.

> **Webapp / facade contract IDs are not reproducible from source.** The web-container signing
> format embeds a monotonic `version` (a timestamp), so signing the same source tree at two
> different moments yields different IDs. Treat the committed `contract-id.txt` / `facade-id.txt` as
> authoritative artifacts, not something to re-derive; CI should check byte-equality on the
> committed `.wasm`, not on the ID.

With reproducible builds in place, a key change becomes a deliberate, reviewable, one-line diff
(bump a pinned version, rebuild) rather than an accident — and _that_ is the point at which you run
a migration.

## Step 2: Preconditions for safe carry-forward

Carrying state forward on Freenet means a client GETs the old state and re-PUTs it under the new
key. Because anyone can PUT, this is only _safe_ if the following hold. An app that lacks them does
**not** get safe carry-forward, and you must design them in before your first release — you cannot
retrofit them onto data that is already live.

- **Mergeable / commutative state.** The new contract must be able to fold the old state into its
  own deterministically. In practice this means your state is a commutative monoid — the same
  requirement that makes Freenet sync work at all (see
  [Contracts](/build/manual/components/contracts/#state-synchronization-and-merging)).
- **Strictly self-authorizing `validate_state`.** The new contract must re-verify _every byte_ it
  accepts, trusting nothing about who delivered it. Every field in state must be covered by a
  signature the validator checks. A permissive validator is not merely sloppy here: during a
  migration it lets a malicious peer win by re-PUTting forged state under the new key, so a weak
  validator turns carry-forward into an attack vector.
- **Key-derived identity, never a contract key.** Anything you hand users as a permanent handle — an
  address, a room reference, a profile link — must be derived from a cryptographic key, not from a
  contract key. The contract key moves on every WASM change; a user-facing identity must not. The
  migration moves _state_ between contract keys while the identity stays fixed.
- **A release-signing key** if you use the successor-pointer path below. The pointer is only
  trustworthy because it is signed by the app author, so you need a signing key you control and keep
  stable across releases.

## Step 3: The migration playbook

The shipped pattern — what River and Delta actually run — is a **backward probe from a committed
registry of legacy code hashes**. You do not need the old WASM bytes, only their hashes.

1. **Keep a registry of every past code hash.** Maintain a committed file (River calls it
   `legacy_contracts.toml` / `legacy_delegates.toml`) listing each historical WASM `code_hash` and
   the stable `params` used with it. A `build.rs` turns it into a `const` the runtime can read.
2. **Reconstruct the old keys.** Because `key = blake3(code_hash ‖ params)` and your `params` are
   stable, the client rebuilds each predecessor key from `(old code_hash, params)` at startup —
   again, with no old WASM in hand.
3. **GET the old state, fold it forward, re-PUT under the new key.** For the first predecessor that
   returns non-empty state, merge it into the current state, **re-run the new contract's
   `validate_state` on the result** (fail closed — if the merge would not validate, abandon it and
   leave your state unchanged), and PUT the validated state under the new key. Because the new
   validator re-checks every signature, any client can do this safely, not only the original author.

**The author-signed pointer is an optional straggler layer, not the mechanism.** Updated clients
already know the new key — their bundled WASM hashes to it. The pointer exists only so that clients
_still running old code_ can discover the successor: the app author signs a pointer from the old key
to the new one and publishes it on the old contract's state (River embeds an `OptionalUpgrade` field
for exactly this). Old clients read it and follow it read-only until they update. It is a
convenience for stragglers layered on top of the backward probe; it is not a substitute for it. Apps
with no shared owner to sign a pointer (per-user state, like a mail inbox) rely on the backward
probe alone.

**Delegates are harder, and more fragile.** A delegate's secrets (signing keys, private user data)
live under its key, so a new delegate WASM makes them invisible. The current mechanism is to keep an
export handler in the delegate _from its very first version_: the new UI asks the old delegate to
export its secrets, the old delegate verifies an author-signed authorization and returns them, and
the UI re-imports them into the new delegate. This works, but it requires **re-running the old
WASM** to read the secrets, which makes it fragile across stdlib and ABI changes — a delegate built
against an old stdlib may not run cleanly under a newer node. Ship the export handler in v1; you
cannot add it retroactively to a delegate that is already holding users' secrets.

## The recommended path: the `freenet-migrate` crate

Hand-rolling all of the above is error-prone — Delta lost data in April 2026 to a secret export that
omitted one variant. The [`freenet-migrate`](https://github.com/freenet/freenet-migrate) crate
(published on crates.io as **v0.1.0**) packages the same machinery River and Delta ship, with the
safety preconditions turned into compiler- and API-enforced guarantees rather than things you have
to remember. Prefer it over rolling your own:

```bash
cargo add freenet-migrate                       # runtime carry-forward
cargo add --build freenet-migrate-build         # build.rs codegen + CI hash-guard
```

What it gives you:

- **`freenet-migrate-build`** generates the predecessor-lineage constants from a `legacy.toml` and
  provides a CI guard that fails the build if the WASM hash changed without the old hash being
  registered — so an accidental re-key cannot reach production silently.
- **`freenet-migrate`** reconstructs predecessor keys (`predecessor_ids`), folds recovered state
  forward through a **fail-closed** `verify()` (a merge that would not validate leaves your state
  untouched), signs and checks the successor pointer (`ReleaseSigner` /
  `verify_and_check_supersedes`, with anti-rollback ordering), and exports delegate secrets
  _generically_ (enumerating them rather than a hand-maintained per-type list, which is exactly the
  omission that cost Delta its data).

**Be honest about its status.** It is `v0.1.0`, targeting current stdlib (0.8.x). The reusable core
and its tests are in place, but the transport that reaches into a predecessor _delegate_ is a
documented stub in this release — it returns `TransportUnavailable`. Today, delegate migration still
works the same way River and Delta do it: the app carries the export across app-side
`DelegateRequest` round-trips, re-running the old WASM. `freenet-migrate` gives you the enforced
preconditions and the contract-side carry-forward now; the node-mediated delegate transport is a
seam for later. See the [repository](https://github.com/freenet/freenet-migrate) and
[freenet-core#2776](https://github.com/freenet/freenet-core/issues/2776) for the full design.

## Pre-release checklist

Before you publish a new contract or delegate version:

- [ ] The stateful contract/delegate is **excluded from the workspace**, has its **own committed
      `Cargo.lock`**, and pins WASM-affecting dependencies with `=x.y.z`.
- [ ] `rust-toolchain.toml` is committed; releases build `--locked` with `-C trim-paths`; the
      `wasm-opt` version is pinned or recorded.
- [ ] A CI guard fails the build if the WASM hash changed without the old `code_hash` being added to
      your legacy registry (`freenet-migrate-build`'s hash-guard, or an equivalent script).
- [ ] State is mergeable/commutative and `validate_state` re-verifies **every** field's signature
      (fail closed).
- [ ] User-facing identities are derived from keys, never from a contract key.
- [ ] Delegates shipped an **export handler in v1**; you have the author signing key to authorize
      migrations and to sign any successor pointer.
- [ ] Every consumer that embeds the WASM (CLI tools, test harnesses) is rebuilt and republished
      together — a stale binary derives the old key and cannot see the new state.
- [ ] You have tested the migration end-to-end on a local node: publish v1, store state/secrets,
      publish v2, confirm the data carries forward.
