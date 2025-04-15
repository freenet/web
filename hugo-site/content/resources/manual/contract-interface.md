---
title: "Contract Interfaces"
date: 2025-04-13
draft: false
---

## Terms

- [Contract State](/manual/glossary#contract-state) - data associated with a contract that can be retrieved by Applications and Delegates.
- [Delta](/manual/glossary#delta) - Represents a modification to some state - similar to a [diff](https://en.wikipedia.org/wiki/Diff) in source code
- [Parameters](/manual/glossary#parameters) - Data that forms part of a contract along with the WebAssembly code
- [State Summary](/manual/glossary#state-summary) - A compact summary of a contract's state that can be used to create a delta

## Interface

Freenet contracts must implement the [`ContractInterface`](https://docs.rs/freenet-stdlib/latest/freenet_stdlib/prelude/trait.ContractInterface.html) trait:

```rust
pub trait ContractInterface {
    // Required methods
    fn validate_state(
        parameters: Parameters<'static>,
        state: State<'static>,
        related: RelatedContracts<'static>,
    ) -> Result<ValidateResult, ContractError>;
    fn update_state(
        parameters: Parameters<'static>,
        state: State<'static>,
        data: Vec<UpdateData<'static>>,
    ) -> Result<UpdateModification<'static>, ContractError>;
    fn summarize_state(
        parameters: Parameters<'static>,
        state: State<'static>,
    ) -> Result<StateSummary<'static>, ContractError>;
    fn get_state_delta(
        parameters: Parameters<'static>,
        state: State<'static>,
        summary: StateSummary<'static>,
    ) -> Result<StateDelta<'static>, ContractError>;
}
```

[`Parameters`](https://docs.rs/freenet-stdlib/latest/freenet_stdlib/prelude/struct.Parameters.html),
[`State`](https://docs.rs/freenet-stdlib/latest/freenet_stdlib/prelude/struct.State.html),
and [`StateDelta`](https://docs.rs/freenet-stdlib/latest/freenet_stdlib/prelude/struct.StateDelta.html)
are all wrappers around simple `[u8]` byte arrays for maximum efficiency and flexibility.

## Contract Interaction

In the (hopefully) near future we'll be adding the ability for contracts to read each other's state while validating and updating their own, see [issue #167](https://github.com/freenet/freenet-core/issues/167) for the latest on this.
