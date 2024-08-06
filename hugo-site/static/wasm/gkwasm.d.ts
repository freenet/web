/* tslint:disable */
/* eslint-disable */
/**
* @param {string} delegate_certificate_base64
* @param {Uint8Array} seed
* @returns {any}
*/
export function wasm_generate_keypair_and_blind(delegate_certificate_base64: string, seed: Uint8Array): any;
/**
* @param {string} delegate_certificate_base64
* @param {string} blinded_signature_base64
* @param {string} blinding_secret_base64
* @param {string} ec_verifying_key_base64
* @param {string} ec_signing_key_base64
* @returns {any}
*/
export function wasm_generate_ghost_key_certificate(delegate_certificate_base64: string, blinded_signature_base64: string, blinding_secret_base64: string, ec_verifying_key_base64: string, ec_signing_key_base64: string): any;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly wasm_generate_keypair_and_blind: (a: number, b: number, c: number, d: number) => number;
  readonly wasm_generate_ghost_key_certificate: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
  readonly __wbindgen_exn_store: (a: number) => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {SyncInitInput} module
*
* @returns {InitOutput}
*/
export function initSync(module: SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {InitInput | Promise<InitInput>} module_or_path
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: InitInput | Promise<InitInput>): Promise<InitOutput>;
