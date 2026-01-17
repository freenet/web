---
title: "Building Decentralized Apps on Freenet"
date: 2025-01-16
draft: false
weight: 1
---

This tutorial teaches you how to build decentralized applications on Freenet. You'll learn the
architecture that makes trustless, peer-to-peer applications possible and how to implement each
component.

**Reference Implementation**: [River](https://github.com/freenet/river) - a decentralized chat
application that demonstrates all the patterns in this tutorial.

---

## 1. Architecture Overview

Freenet applications have three components that work together. See the
[Components Overview](/resources/manual/components/overview/) for a detailed diagram.

### Contract (Network Layer)

The contract is your application's backend. It runs as WebAssembly on untrusted peers across the
network.

**Key properties:**
- Defines what valid state looks like and how it can be modified
- Runs on peers you don't control—assume it's adversarial
- Cannot store private keys (anyone can read the code and state)
- The contract's key is the hash of its WASM code

**Example**: In River, the contract stores room membership, messages, and configuration.

### Delegate (Local Trust Zone)

The delegate runs locally on the user's device inside the Freenet Kernel. This is your secure
execution environment.

**Key properties:**
- Stores private keys and secrets
- Performs cryptographic operations (signing, encryption)
- Can run background tasks
- Never exposed to the network

**Example**: In River, the delegate manages the user's signing keys and encrypts messages for
private rooms.

### UI (Frontend)

A standard web application that connects to the local Freenet Kernel via WebSocket.

**Key properties:**
- Built with any web framework (River uses [Dioxus](https://dioxuslabs.com))
- Communicates with contracts through the kernel API
- Can be served as a Freenet contract itself (web container)

---

## 2. The Consistency Model

Freenet is a distributed system where peers may receive updates in different orders. Your contract
must handle this correctly.

### Commutative Monoids

Contract state must form a **commutative monoid**—updates can be applied in any order and still
produce the same final state.

![Commutative synchronization between peers](/images/tutorial/commutative-sync.svg)

Both peers end up with the same state regardless of which update they received first.

### Summaries and Deltas

Instead of transferring complete state, peers exchange:

- **Summary**: A compact representation of what a peer has (e.g., hashes, version numbers)
- **Delta**: The minimal update needed to bring another peer up to date

This makes synchronization efficient even for large states.

---

## 3. Prerequisites

### Rust Toolchain

```bash
# Install Rust via rustup (not Homebrew on macOS)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add WebAssembly target
rustup target add wasm32-unknown-unknown
```

### Freenet

```bash
# Clone and build from source
git clone https://github.com/freenet/freenet-core.git
cd freenet-core
cargo install --path crates/core
```

### Build Tools

```bash
cargo install cargo-make
```

---

## 4. Project Structure

A Freenet application is organized as a Cargo workspace:

```
my-app/
├── Cargo.toml              # Workspace definition
├── Makefile.toml           # Build tasks
├── common/                 # Shared types and logic
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs
├── contracts/
│   └── my-contract/        # Contract implementation
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs
├── delegates/
│   └── my-delegate/        # Delegate implementation
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs
└── ui/                     # Web frontend
    ├── Cargo.toml
    └── src/
        └── main.rs
```

### Workspace Cargo.toml

```toml
[workspace]
members = [
    "common",
    "contracts/my-contract",
    "delegates/my-delegate",
    "ui",
]
resolver = "2"

[workspace.dependencies]
# Serialization
ciborium = "0.2.2"
serde = { version = "1.0", features = ["derive"] }

# Freenet
freenet-scaffold = "0.2.1"
freenet-scaffold-macro = "0.2.1"
freenet-stdlib = { version = "0.1.30", features = ["contract"] }

# Your shared types
my-app-common = { path = "common" }
```

---

## 5. Contract Development

Contracts define your application's shared state and the rules for modifying it.

### Using freenet-scaffold

The `freenet-scaffold` crate provides the `#[composable]` macro that automatically generates the
summary, delta, merge, and verify functions your state needs.

```rust
// common/src/lib.rs
use freenet_scaffold_macro::composable;
use serde::{Deserialize, Serialize};

#[composable]
#[derive(Serialize, Deserialize, Clone, Default, PartialEq, Debug)]
pub struct AppState {
    pub configuration: Configuration,
    pub members: Members,
    pub messages: Messages,
}

#[derive(Serialize, Deserialize, Clone, Default, PartialEq, Debug)]
pub struct AppParameters {
    pub owner: VerifyingKey,
}
```

The `#[composable]` macro generates:
- `AppStateSummary` - compact state representation
- `AppStateDelta` - incremental updates
- `ComposableState` trait implementation with `summarize`, `delta`, `apply_delta`, `merge`, and
  `verify` methods

### Contract Implementation

```rust
// contracts/my-contract/src/lib.rs
use ciborium::{de::from_reader, ser::into_writer};
use freenet_stdlib::prelude::*;
use freenet_scaffold::ComposableState;
use my_app_common::{AppState, AppStateDelta, AppStateSummary, AppParameters};

struct Contract;

#[contract]
impl ContractInterface for Contract {
    fn validate_state(
        parameters: Parameters<'static>,
        state: State<'static>,
        _related: RelatedContracts<'static>,
    ) -> Result<ValidateResult, ContractError> {
        if state.as_ref().is_empty() {
            return Ok(ValidateResult::Valid);
        }

        let app_state: AppState = from_reader(state.as_ref())
            .map_err(|e| ContractError::Deser(e.to_string()))?;
        let params: AppParameters = from_reader(parameters.as_ref())
            .map_err(|e| ContractError::Deser(e.to_string()))?;

        app_state
            .verify(&app_state, &params)
            .map(|_| ValidateResult::Valid)
            .map_err(|_| ContractError::InvalidState)
    }

    fn update_state(
        parameters: Parameters<'static>,
        state: State<'static>,
        data: Vec<UpdateData<'static>>,
    ) -> Result<UpdateModification<'static>, ContractError> {
        let params: AppParameters = from_reader(parameters.as_ref())
            .map_err(|e| ContractError::Deser(e.to_string()))?;
        let mut app_state: AppState = from_reader(state.as_ref())
            .map_err(|e| ContractError::Deser(e.to_string()))?;

        for update in data {
            match update {
                UpdateData::Delta(d) => {
                    let delta: AppStateDelta = from_reader(d.as_ref())
                        .map_err(|e| ContractError::Deser(e.to_string()))?;
                    app_state
                        .apply_delta(&app_state.clone(), &params, &Some(delta))
                        .map_err(|_| ContractError::InvalidUpdate)?;
                }
                UpdateData::State(new_state) => {
                    let new: AppState = from_reader(new_state.as_ref())
                        .map_err(|e| ContractError::Deser(e.to_string()))?;
                    app_state
                        .merge(&app_state.clone(), &params, &new)
                        .map_err(|_| ContractError::InvalidUpdate)?;
                }
                _ => {}
            }
        }

        let mut output = vec![];
        into_writer(&app_state, &mut output)
            .map_err(|e| ContractError::Deser(e.to_string()))?;
        Ok(UpdateModification::valid(output.into()))
    }

    fn summarize_state(
        parameters: Parameters<'static>,
        state: State<'static>,
    ) -> Result<StateSummary<'static>, ContractError> {
        if state.as_ref().is_empty() {
            return Ok(StateSummary::from(vec![]));
        }
        let params: AppParameters = from_reader(parameters.as_ref())
            .map_err(|e| ContractError::Deser(e.to_string()))?;
        let app_state: AppState = from_reader(state.as_ref())
            .map_err(|e| ContractError::Deser(e.to_string()))?;

        let summary = app_state.summarize(&app_state, &params);
        let mut output = vec![];
        into_writer(&summary, &mut output)
            .map_err(|e| ContractError::Deser(e.to_string()))?;
        Ok(StateSummary::from(output))
    }

    fn get_state_delta(
        parameters: Parameters<'static>,
        state: State<'static>,
        summary: StateSummary<'static>,
    ) -> Result<StateDelta<'static>, ContractError> {
        let params: AppParameters = from_reader(parameters.as_ref())
            .map_err(|e| ContractError::Deser(e.to_string()))?;
        let app_state: AppState = from_reader(state.as_ref())
            .map_err(|e| ContractError::Deser(e.to_string()))?;
        let summary: AppStateSummary = from_reader(summary.as_ref())
            .map_err(|e| ContractError::Deser(e.to_string()))?;

        let delta = app_state.delta(&app_state, &params, &summary);
        let mut output = vec![];
        into_writer(&delta, &mut output)
            .map_err(|e| ContractError::Deser(e.to_string()))?;
        Ok(StateDelta::from(output))
    }
}
```

### Contract Cargo.toml

```toml
[package]
name = "my-contract"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
ciborium = { workspace = true }
freenet-stdlib = { workspace = true }
freenet-scaffold = { workspace = true }
my-app-common = { workspace = true }
```

---

## 6. Delegate Development

Delegates handle secrets and run locally. They're optional—simple apps may not need them.

```rust
// delegates/my-delegate/src/lib.rs
use freenet_stdlib::prelude::*;

struct Delegate;

#[delegate]
impl DelegateInterface for Delegate {
    fn process(
        _params: Parameters<'static>,
        _attested: Option<&'static [u8]>,
        messages: InboundDelegateMsg,
    ) -> Result<Vec<OutboundDelegateMsg>, DelegateError> {
        match messages {
            InboundDelegateMsg::UserResponse(response) => {
                // Handle user input
                Ok(vec![])
            }
            InboundDelegateMsg::ApplicationMessage(app_id, msg) => {
                // Handle messages from UI
                Ok(vec![])
            }
            _ => Ok(vec![]),
        }
    }
}
```

---

## 7. UI Development

The UI connects to the local Freenet Kernel via WebSocket to interact with contracts.

### WebSocket Connection

```typescript
const API_URL = `ws://${location.host}/contract/command/`;

const handler = {
    onContractPut: (response) => { /* Contract created */ },
    onContractGet: (response) => { /* State received */ },
    onContractUpdate: (response) => { /* State updated */ },
    onErr: (err) => console.error(err),
    onOpen: () => console.log("Connected to Freenet"),
};

const api = new FreenetWsApi(API_URL, handler);

// Subscribe to a contract
await api.subscribe({ key: contractKey });

// Send an update
await api.update({
    key: contractKey,
    delta: encodedDelta,
});
```

### Using Dioxus (Rust)

River uses [Dioxus](https://dioxuslabs.com) for a fully Rust-based UI that compiles to WebAssembly:

```rust
use dioxus::prelude::*;

fn App() -> Element {
    let messages = use_signal(Vec::new);

    rsx! {
        div {
            for msg in messages.read().iter() {
                p { "{msg}" }
            }
        }
    }
}
```

---

## 8. Building and Testing

### Build Configuration (Makefile.toml)

```toml
[tasks.build]
description = "Build all components"
dependencies = ["build-contracts", "build-ui"]

[tasks.build-contracts]
command = "cargo"
args = ["build", "--release", "--target", "wasm32-unknown-unknown", "-p", "my-contract"]

[tasks.build-ui]
command = "dx"
args = ["build", "--release"]
cwd = "ui"

[tasks.test]
command = "cargo"
args = ["test", "--workspace"]
```

### Local Testing

```bash
# Start Freenet in local mode (no network)
freenet local

# In another terminal, build and publish your contract
cargo make build
```

### Running River for Reference

```bash
git clone https://github.com/freenet/river.git
cd river
git submodule init && git submodule update

# Run with example data (no Freenet needed)
cargo make dev-example

# Open http://localhost:8080
```

---

## 9. Deployment

### Publishing a Contract

After building, publish your contract to the network:

```bash
# The contract WASM is in target/wasm32-unknown-unknown/release/
freenet publish \
    --code target/wasm32-unknown-unknown/release/my_contract.wasm \
    --state initial_state.cbor
```

The command returns a contract key (hash of the WASM) that users need to access your application.

### Web Container

To serve your UI over Freenet, wrap it in a web container contract. See
[River's web-container-contract](https://github.com/freenet/river/tree/main/contracts/web-container-contract)
for an example.

---

## 10. Next Steps

1. **Study River**: The [River repository](https://github.com/freenet/river) is a complete,
   production application. Read its code to understand real-world patterns.

2. **Read the scaffold docs**: The
   [freenet-scaffold](https://github.com/freenet/freenet-scaffold) crate documentation explains
   composable state in detail.

3. **Join the community**: Get help and share your work in the
   [official River room](https://freenet.org/quickstart/).

---

## Current Limitations

- **Network deployment**: The public Freenet network is under active development
- **Language support**: Contracts must be written in Rust (compiles to WASM)
- **Tooling**: Build from source; packaged binaries coming soon
