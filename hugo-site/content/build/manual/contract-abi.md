---
title: "Contract ABI (Non-Rust Contracts)"
date: 2026-07-27
draft: false
---

Contracts are WebAssembly, so any language that compiles to `wasm32` can implement one. Rust is the
only language with a ready-made binding today, and the [`freenet-stdlib`][stdlib] macro hides the
whole boundary, so nothing states the raw ABI in one place. This page does that.

If you are writing a contract in Rust, you do not need any of this. Use
[Contract Interfaces](/build/manual/contract-interface/) instead.

{{< alert type="warning" >}} **No non-Rust contract has been written yet**, so most of this boundary
has only ever had one caller. Expect rough edges, and please
[report anything that does not match](https://github.com/freenet/freenet-core/issues).
{{< /alert >}}

## Exports your module must provide

| Export                    | Signature                | Required                     |
| ------------------------- | ------------------------ | ---------------------------- |
| `memory`                  | linear memory            | yes, under exactly this name |
| `__frnt__initiate_buffer` | `(i32) -> i64`           | yes                          |
| `validate_state`          | `(i64, i64, i64) -> i64` | yes                          |
| `update_state`            | `(i64, i64, i64) -> i64` | yes                          |
| `summarize_state`         | `(i64, i64) -> i64`      | yes                          |
| `get_state_delta`         | `(i64, i64, i64) -> i64` | yes                          |
| `__frnt_set_id`           | `(i64) -> ()`            | only if you use host imports |

The host looks up `__frnt_set_id` with an optional lookup and skips it when absent, so a contract
that never calls back into the host can leave it out.

Argument order matches the Rust trait:

- `validate_state(parameters, state, related)`
- `update_state(parameters, state, update_data)`
- `summarize_state(parameters, state)`
- `get_state_delta(parameters, state, summary)`

## Pointer convention

Every `i64` crossing the boundary is a `wasm32` linear-memory offset widened to 64 bits. The host
adds its own memory base when dereferencing. From inside the module they are ordinary 32-bit
pointers, so a widening cast is all that is needed.

## Buffer protocol

There are two, and your module selects one by what it imports. The host checks whether the module
imports anything at all from the `freenet_contract_io` namespace. Import nothing from it and you get
the legacy path.

### Legacy one-shot

Start here. The host calls `__frnt__initiate_buffer(len)` with the exact payload length, then writes
the whole payload at `start`. There is no length header, no chunking and no refill callback. Your
entry point receives a pointer to the `BufferBuilder` and the payload is the first `capacity` bytes
at `start`.

This path has no size cap and is fully supported.

### Streaming

Import `freenet_contract_io.__frnt__fill_buffer` and the host switches to a chunked protocol capped
at 64 KiB per buffer. The buffer then begins with a 4-byte little-endian `u32` giving the total
payload length, followed by as much data as fits. When you have consumed what is present, call:

```
__frnt__fill_buffer(instance_id: i64, buf_ptr: i64) -> u32
```

It returns the number of bytes written into the buffer, or `0` for end of input. `instance_id` is
the value handed to you by `__frnt_set_id`, which becomes mandatory on this path.

Only worth adopting once the basics work.

## Struct layouts

Both structs are C-representation as seen by `wasm32`, where `i64` has alignment 8.

`BufferBuilder` is 32 bytes, alignment 8. `__frnt__initiate_buffer` returns a pointer to one:

| Offset | Field        | Type                            |
| ------ | ------------ | ------------------------------- |
| 0      | `start`      | `i64`, pointer to the data      |
| 8      | `capacity`   | `u32`                           |
| 12     | (padding)    | 4 bytes                         |
| 16     | `last_read`  | `i64`, pointer to a `u32` count |
| 24     | `last_write` | `i64`, pointer to a `u32` count |

`last_read` and `last_write` are pointers to `u32` read and write positions, not the positions
themselves. That detail is easy to miss.

`ContractInterfaceResult` is 16 bytes, alignment 8. Your entry point returns a pointer to one:

| Offset | Field  | Type                                    |
| ------ | ------ | --------------------------------------- |
| 0      | `ptr`  | `i64`, pointer to the serialized result |
| 8      | `kind` | `i32`                                   |
| 12     | `size` | `u32`, byte length of the result        |

`kind` values are `0` ValidateState, `1` ValidateDelta, `2` UpdateState, `3` SummarizeState and `4`
StateDelta. The host rejects anything else, and it checks that the kind matches the entry point it
called.

## Payload encoding

Payloads use bincode 1.x through its free `serialize` and `deserialize` functions. That means fixed
integer encoding, little-endian, 8-byte lengths and 4-byte enum discriminants. It is a different
layout from the variable-length integer configuration that bincode's options builder produces, and
different again from bincode 2.x. Getting this wrong is the most likely source of silent breakage.

What carries bincode framing:

- Incoming: `related` is a `RelatedContracts`, and `update_data` is a list of `UpdateData`. The
  `parameters`, `state` and `summary` arguments are raw bytes with no framing.
- Outgoing: all four entry points return a bincode-encoded `Result`, wrapping `ValidateResult`,
  `UpdateModification`, `StateSummary` and `StateDelta` respectively, with `ContractError` as the
  error type.

`UpdateData` and `ContractError` are both marked non-exhaustive upstream, so variant indices can
gain entries. Do not assume the current count is final.

## Host imports available

Each takes the instance id from `__frnt_set_id` as its first argument.

| Namespace             | Function                   | Signature           |
| --------------------- | -------------------------- | ------------------- |
| `freenet_log`         | `__frnt__logger__info`     | `(i64, i64, i32)`   |
| `freenet_rand`        | `__frnt__rand__rand_bytes` | `(i64, i64, u32)`   |
| `freenet_time`        | `__frnt__time__utc_now`    | `(i64, i64)`        |
| `freenet_contract_io` | `__frnt__fill_buffer`      | `(i64, i64) -> u32` |

Importing anything from `freenet_contract_io` opts you into the streaming protocol, so do not import
it speculatively.

## Memory management

The Rust implementation leaks everything it returns, deliberately. Buffers from
`__frnt__initiate_buffer` and the result struct are both leaked, and the runtime reclaims them by
tearing down the whole instance after the call. A bump allocator that never frees is the right
design here. Freeing a result before the host reads it is a use-after-free.

## Behavioural requirement

`update_state` must be commutative with respect to deltas. Applying a set of deltas in any order has
to converge on the same state. The network deprioritizes contracts that violate this, so it is a
correctness requirement rather than a style note. See
[Delta-Sync](/build/manual/further-reading/delta-sync/) for the reasoning.

## Reading the source

The authoritative definition is the host code that loads and calls the module.

In [freenet-core](https://github.com/freenet/freenet-core):

- [`wasm_runtime/contract.rs`][core-contract] is the call sequence for all four entry points, and
  the best starting point.
- [`wasm_runtime/runtime.rs`][core-runtime] holds `write_streaming_buf` and `write_contract_buf`,
  which show exactly what the host places in linear memory before calling.
- [`engine/wasmtime_engine.rs`][core-engine] contains the streaming-versus-legacy detection.

In [freenet-stdlib](https://github.com/freenet/freenet-stdlib):

- [`memory/buf.rs`][stdlib-buf] defines `BufferBuilder`.
- [`contract_interface/wasm_interface.rs`][stdlib-wasm] defines `ResultKind` and
  `ContractInterfaceResult`. Only the type definitions near the top are relevant; the rest is
  host-side decoding and Rust macro support.
- [`contract_interface/update.rs`][stdlib-update], [`state.rs`][stdlib-state] and
  [`error.rs`][stdlib-error] define the encoded types.

[stdlib]: https://github.com/freenet/freenet-stdlib
[core-contract]:
  https://github.com/freenet/freenet-core/blob/main/crates/core/src/wasm_runtime/contract.rs
[core-runtime]:
  https://github.com/freenet/freenet-core/blob/main/crates/core/src/wasm_runtime/runtime.rs
[core-engine]:
  https://github.com/freenet/freenet-core/blob/main/crates/core/src/wasm_runtime/engine/wasmtime_engine.rs
[stdlib-buf]: https://github.com/freenet/freenet-stdlib/blob/main/rust/src/memory/buf.rs
[stdlib-wasm]:
  https://github.com/freenet/freenet-stdlib/blob/main/rust/src/contract_interface/wasm_interface.rs
[stdlib-update]:
  https://github.com/freenet/freenet-stdlib/blob/main/rust/src/contract_interface/update.rs
[stdlib-state]:
  https://github.com/freenet/freenet-stdlib/blob/main/rust/src/contract_interface/state.rs
[stdlib-error]:
  https://github.com/freenet/freenet-stdlib/blob/main/rust/src/contract_interface/error.rs
