---
title: "TypeScript SDK"
date: 2026-06-05
draft: false
---

[`@freenetorg/freenet-stdlib`](https://www.npmjs.com/package/@freenetorg/freenet-stdlib) is the
official TypeScript/JavaScript SDK for talking to a Freenet node from a browser or Node.js app. It
opens a WebSocket to the node and exposes a promise-based API for the contract and delegate
operations.

> **This is a client SDK.** It connects a user interface to a node; it is not used to author
> contracts or delegates. To write contracts and delegates (in Rust), see
> [Contract Interfaces](/build/manual/contract-interface/) and the
> [`freenet-stdlib` API on docs.rs](https://docs.rs/freenet-stdlib). For the Rust UI client
> (Dioxus), use `freenet-stdlib` with the `net` feature.

```bash
npm install @freenetorg/freenet-stdlib
```

No version pin is needed; npm installs the current release. In Node.js, also install the optional
`ws` package (`npm install ws`); browsers provide their own WebSocket.

This page is the API reference. For the end-to-end app workflow see the
[tutorial](/build/manual/tutorial/), and for the UI-side overview see
[User Interface](/build/manual/components/ui/). [Raven](https://github.com/freenet/raven) (a
decentralized microblogging app) is a complete TypeScript + Vite reference.

## Connecting: `FreenetWsApi`

```typescript
import { FreenetWsApi, ResponseHandler } from "@freenetorg/freenet-stdlib";

const api = new FreenetWsApi(url, handler, authToken);
```

| Argument    | Type                | Notes                                                                                                                      |
| ----------- | ------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| `url`       | `URL`               | The node's command WebSocket. Derive it from `location`: `new URL(\`ws://${location.host}/v1/contract/command\`)`.         |
| `handler`   | `ResponseHandler`   | Callback object the node pushes results and live notifications to.                                                         |
| `authToken` | `string` (optional) | Leave empty (`""`) inside the Freenet web-container shell; the shell injects auth. Set it only for standalone/CLI clients. |

The request methods are promise-based and also fire the matching `handler` callback. The default
request timeout is 30 seconds.

```typescript
class FreenetWsApi {
  async put(put: PutRequest): Promise<PutResponse>;
  async update(update: UpdateRequest): Promise<UpdateResponse>;
  async get(get: GetRequest): Promise<GetResponse>;
  async subscribe(subscribe: SubscribeRequest): Promise<void>;
  async disconnect(disconnect: DisconnectRequest): Promise<void>;
}
```

## The `ResponseHandler`

The node delivers results and subscription updates through this callback object. Required methods
are unmarked; optional ones are noted.

```typescript
interface ResponseHandler {
  onContractPut(response: PutResponse): void;
  onContractGet(response: GetResponse): void;
  onContractUpdate(response: UpdateResponse): void;
  onContractUpdateNotification(response: UpdateNotification): void; // live updates from subscribe()
  onContractNotFound(instanceId: ContractInstanceId): void;
  onSubscribeResponse?(key: ContractKey, subscribed: boolean): void; // optional
  onDelegateResponse(response: DelegateResponse): void;
  onErr(response: HostError): void;
  onOpen(): void;
  onClose?(code: number, reason: string): void; // optional
}
```

## Requests

| Class               | Constructor                                                                      | Purpose                                                           |
| ------------------- | -------------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| `GetRequest`        | `(key, fetchContract?, subscribe?, blockingSubscribe?)`                          | Fetch contract state (and optionally the contract code).          |
| `PutRequest`        | `(container?, wrappedState?, relatedContracts?, subscribe?, blockingSubscribe?)` | Publish/deploy a contract with its initial state.                 |
| `UpdateRequest`     | `(key?, update?)`                                                                | Push a delta or state update.                                     |
| `SubscribeRequest`  | `(key?, summary?)`                                                               | Subscribe to a contract's updates.                                |
| `DisconnectRequest` | `(cause?)`                                                                       | Disconnect gracefully.                                            |
| `DelegateRequest`   | `(type, request)`                                                                | Register/unregister a delegate or send it an application message. |

## Responses

| Class                | Key fields                                                           |
| -------------------- | -------------------------------------------------------------------- |
| `PutResponse`        | `key: ContractKey`                                                   |
| `GetResponse`        | `key: ContractKey`, `contract: ContractContainer`, `state: number[]` |
| `UpdateResponse`     | `key: ContractKey`, `summary: number[]`                              |
| `UpdateNotification` | `key: ContractKey`, `update: UpdateData`                             |
| `DelegateResponse`   | `key?: DelegateKey`, `values?: OutboundDelegateMsg[]`                |
| `HostError`          | `{ cause: string }`                                                  |

## Request ordering

The SDK resolves `get()` promises from a single FIFO queue with no request correlation: the Nth
`GetResponse` (or not-found) to arrive settles the Nth pending `get()`, regardless of which contract
it was for. Do not run several `get()` calls concurrently, or responses will be delivered to the
wrong caller. Serialize reads instead: await one before starting the next, or chain them. Raven does
this with a `getChain` promise; see its
[`web/src/freenet-api.ts`](https://github.com/freenet/raven/blob/main/web/src/freenet-api.ts).

## Keys and containers

```typescript
import {
  ContractKey,
  ContractContainer,
  ContractType,
  WasmContractV1,
} from "@freenetorg/freenet-stdlib";

// A contract key from a base58 instance id (as printed by `fdev`)
const key = ContractKey.fromInstanceId("DCBi7HNZC3QUZRiZLFZDiEduv5KHgZfgBk8WwTiheGq1");
key.encode(); // base58 string

// Wrap WASM + parameters for a PutRequest
const contract = new WasmContractV1(contractCode, parameterBytes, key);
const container = new ContractContainer(ContractType.WasmContractV1, contract);
```

- `ContractInstanceId`: the 32-byte contract hash (a `Uint8Array`).
- `ContractKey`: wraps a `ContractInstanceId`; methods include `fromInstanceId()`, `bytes()`,
  `codePart()`, `encode()`. Note that `fromInstanceId()` sets only the instance part of the key; the
  node needs both parts, so for a contract with no separate parameters re-wrap it as
  `new ContractKey(bytes, bytes)` (as Raven does).
- `ContractContainer` / `WasmContractV1`: carry the contract WASM and its parameters for a `put`.

## Update data

`UpdateData` is a discriminated union built with `UpdateDataType` plus one of six concrete update
types:

```typescript
import { UpdateData, UpdateDataType, DeltaUpdate } from "@freenetorg/freenet-stdlib";

const deltaBytes = new TextEncoder().encode(JSON.stringify(myDelta));
const update = new UpdateData(UpdateDataType.DeltaUpdate, new DeltaUpdate(Array.from(deltaBytes)));
await api.update(new UpdateRequest(key, update));
```

| Class                        | Constructor                    |
| ---------------------------- | ------------------------------ |
| `StateUpdate`                | `(state?)`                     |
| `DeltaUpdate`                | `(delta?)`                     |
| `StateAndDeltaUpdate`        | `(state?, delta?)`             |
| `RelatedStateUpdate`         | `(relatedTo?, state?)`         |
| `RelatedDeltaUpdate`         | `(relatedTo?, delta?)`         |
| `RelatedStateAndDeltaUpdate` | `(relatedTo?, state?, delta?)` |

## Delegates

Message a delegate with `DelegateRequest` carrying `ApplicationMessages`. Messages your app sends
into the delegate (`InboundDelegateMsg`) are `ApplicationMessage` / `UserInputResponse`; messages
the delegate emits back (`OutboundDelegateMsg`, delivered in `DelegateResponse.values`) are
`ApplicationMessage` / `RequestUserInput` / `ContextUpdated`.

To register or remove a delegate, wrap it in a `DelegateContainer` (`WasmDelegateV1`) and send a
`DelegateRequest` with `RegisterDelegate` / `UnregisterDelegate`. Delegate secret storage uses the
`GetSecretRequest`, `SetSecretRequest`, and `GetSecretResponse` types.

These are the lower-level FlatBuffers types and live under the package subpaths
(`@freenetorg/freenet-stdlib/common`, `/client-request`, `/host-response`); this part of the API is
still stabilizing. Raven's
[`web/src/delegate-api.ts`](https://github.com/freenet/raven/blob/main/web/src/delegate-api.ts)
shows the current working approach for building and sending delegate messages.

## Streaming large state

Large state and deltas (above ~512 KB) are split into chunks on the wire. **`FreenetWsApi`
reassembles incoming chunks automatically** and delivers the complete payload through the normal
`ResponseHandler` callbacks, and it chunks large outgoing requests for you, so you do not normally
handle streaming yourself.

The `streaming` module is exported for advanced or low-level use:

- `ReassemblyBuffer`: manual chunk reassembly. `receiveChunk(streamId, index, total, data)` returns
  the full `Uint8Array` once every chunk has arrived, otherwise `null`.
- `StreamError`: thrown on a protocol violation (zero or too many chunks, index out of range, stream
  timeout).
- Sizing constants: `CHUNK_SIZE` (256 KB), `CHUNK_THRESHOLD` (512 KB), `MAX_TOTAL_CHUNKS` (256),
  `MAX_CONCURRENT_STREAMS` (8).

## Packaging for a web container

When your UI is served from a Freenet web container it runs in a sandboxed iframe with a strict
Content Security Policy. Set `base: "./"` in your Vite config so assets resolve relative to the
container URL, and prefer vendoring third-party CSS/JS locally; the gateway applies a Content
Security Policy, so verify any remote origins against a real gateway. See
[User Interface](/build/manual/components/ui/) for details.
