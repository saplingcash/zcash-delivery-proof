# Changelog

## 0.1.0

First release, from GitHub only (not yet on crates.io or npm).

- Delivery proofs (`zdp:1:`) for Orchard and Ironwood notes: `make` with a unified full viewing key
  (notes received, and notes sent through the outgoing viewing key) or a unified incoming viewing key
  (notes received); `check` with no key.
- The `verify-bundle` feature (off by default): the bundle's zk-SNARK and signatures, offline.
- A WebAssembly package (`wasm/pkg`), rebuilt byte for byte by `wasm/build.sh` from pinned inputs.
- Test vectors: Orchard (v5) and Ironwood (v6) transactions built from label-derived keys, and a real
  transaction mined on Zcash testnet.
