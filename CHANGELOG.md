# Changelog

## 0.2.0

Sapling notes.

- Delivery proofs for notes in the Sapling pool (pool byte `3`; the proof format and its `zdp:1:` text are
  otherwise unchanged): `make` finds them with a UFVK's or UIVK's Sapling item (received) or a Sapling
  outgoing viewing key (sent), and `check` verifies them with no key, as for Orchard and Ironwood
  (SPEC.md §2 to §4). Only notes with plaintext version 2 (ZIP 212) can be proved.
- `ViewingKeys::from_outgoing_keys`: outgoing viewing keys alone, as 32 bytes, for a sender that publishes
  one so that anyone can prove what it sent. `ViewingKeys::none` and `from_sapling_full_viewing_key`.
- `pool_receiver_address` and `address_has_pool_receiver`: the receiver of any pool as a unified address,
  and the match against a payer's address (a Sapling receiver also matches a plain Sapling address).
- The WebAssembly package: `make` and `addressHasReceiver` cover Sapling; `makeWithOutgoingKeys`.
- `verify_bundle` (feature `verify-bundle`) refuses a Sapling bundle with `Error::Unsupported`.
- Test vectors: a Sapling case in `constructed.json` (placeholder Output proofs, which no check reads),
  and `testnet-sapling.json`, a real Sapling payment mined on Zcash testnet.

Breaking, for code written against 0.1.0: `ViewingKeys` has two more fields (`sapling_incoming`,
`sapling_outgoing`; struct literals need them, or start from `ViewingKeys::none`), `Pool` has a third
variant and `Error` a fifth (`Unsupported`), so exhaustive matches need an arm for each. A UFVK or UIVK
with a Sapling item and no Orchard item, refused before, is now read.

## 0.1.0

First release, from GitHub only (not yet on crates.io or npm).

- Delivery proofs (`zdp:1:`) for Orchard and Ironwood notes: `make` with a unified full viewing key
  (notes received, and notes sent through the outgoing viewing key) or a unified incoming viewing key
  (notes received); `check` with no key.
- The `verify-bundle` feature (off by default): the bundle's zk-SNARK and signatures, offline.
- A WebAssembly package (`wasm/pkg`), rebuilt byte for byte by `wasm/build.sh` from pinned inputs.
- Test vectors: Orchard (v5) and Ironwood (v6) transactions built from label-derived keys, and real
  Ironwood transactions mined on Zcash testnet and mainnet. There is no mainnet Orchard vector yet.
