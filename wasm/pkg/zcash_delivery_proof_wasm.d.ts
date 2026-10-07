/* tslint:disable */
/* eslint-disable */

/**
 * Whether an address (a unified address as a payer was given it, or a Sapling address) carries the
 * receiver a proof names, in the proof's pool.
 */
export function addressHasReceiver(address: string, proof: string): boolean;

/**
 * Checks a proof (`zdp:1:…`) against the transaction's bytes (hex) with no key. Returns the delivery as
 * JSON: txid, wtxid, pool, action, address (in `network`'s encoding), value, memoHex, memoText.
 */
export function check(tx_hex: string, proof: string, network_name: string): string;

/**
 * Every note a UFVK (received and sent) or UIVK (received) sees in the transaction (Orchard, Ironwood and
 * Sapling), each with its proof, as a JSON array. The key's own network chooses the address encoding.
 */
export function make(tx_hex: string, viewing_key: string): string;

/**
 * Every note the given outgoing viewing keys sent in the transaction, each with its proof, as a JSON
 * array (as `make`). `orchardOvks` and `saplingOvks`: comma-separated, 32 bytes of hex each, either may
 * be empty. For a sender that publishes its outgoing viewing key, so that anyone can prove what it sent.
 */
export function makeWithOutgoingKeys(tx_hex: string, network_name: string, orchard_ovks: string, sapling_ovks: string): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly addressHasReceiver: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly check: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number, number];
    readonly make: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly makeWithOutgoingKeys: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number, number];
    readonly rustsecp256k1_v0_14_context_create: (a: number) => number;
    readonly rustsecp256k1_v0_14_context_destroy: (a: number) => void;
    readonly rustsecp256k1_v0_14_default_error_callback_fn: (a: number, b: number) => void;
    readonly rustsecp256k1_v0_14_default_illegal_callback_fn: (a: number, b: number) => void;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
