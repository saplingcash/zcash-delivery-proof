# zcash-delivery-proof

Prove that a Zcash shielded payment was delivered to an address, with its memo, without revealing the
wallet.

A **delivery proof** is one short string (`zdp:1:…`). It says: this transaction delivered a note of this
value, with this memo, to this address, in the Orchard or the Ironwood pool.
- **Who makes it:** the receiving wallet, with its incoming viewing key, or the sending wallet, with its
  outgoing viewing key.
- **Who checks it:** anyone, against the transaction's bytes, with no key at all.
- **What it reveals:** that one note, and no viewing key or other payment.

Uses:
- **Receipts:** a shop shows a customer or an accountant that a payment arrived, with its memo.
- **Invoices:** a customer whose payment is disputed proves, from its own wallet, that it paid the
  invoice's address the invoiced amount, quoting the invoice.
- **Audits:** an organisation proves chosen incoming payments to an auditor, one proof each, instead of
  handing over a viewing key that reveals every payment.

The format and the checks are in [SPEC.md](SPEC.md). A Rust crate and a WebAssembly package for browsers
and Node make and check proofs.

## What a check shows, and what it does not

A check rebuilds the note from the proof and requires three things of the transaction's bytes:
- its action commits to exactly that note (the note commitment);
- the note's own key decrypts the action to it: the same receiver, value and rseed;
- the memo comes out of that authenticated encryption.

It does not show:
- **that the transaction is mined:** look its txid up on a node or an explorer;
- **who sent it;**
- **that the bundle's zk-SNARK and signatures hold:** the `verify-bundle` feature checks those offline.

A proof reveals its receiver (one diversified address), value and memo. It does not reveal whether the
note was later spent. See SPEC.md §5.

## Rust

Until it is on crates.io, depend on the repository at a fixed commit:

```toml
[dependencies]
zcash-delivery-proof = { git = "https://github.com/saplingcash/zcash-delivery-proof", rev = "<commit>" }
```

```rust
use zcash_delivery_proof::{check, make, memo_text, DeliveryProof, ViewingKeys};

// a wallet: every note its key received or sent in the transaction, each with a proof
let keys = ViewingKeys::parse(ufvk)?;              // a UFVK (received and sent) or a UIVK (received)
for found in make(&tx_bytes, &keys)? {
    println!("{:?} {}", found.side, found.proof.encode());
}

// anyone: check a proof against the transaction's bytes
let d = check(&tx_bytes, &DeliveryProof::decode(proof_text)?)?;
println!("{} zatoshi, memo {:?}, in {}", d.value, memo_text(&d.memo), d.txid_hex());
```

It uses the Zcash crates' own transaction parser, note encryption and key types (zcash_primitives 0.30,
orchard 0.15). The optional `verify-bundle` feature adds `verify_bundle(tx, pool, spent_coins)`. It builds
the verifying key the first time it is called: seconds and tens of MB.

Examples, running on the test vectors:

```sh
cargo run --example receipt
cargo run --example invoice
cargo run --example audit
```

## WebAssembly (browser and Node)

`wasm/pkg` holds the package, built by `wasm/build.sh`:

```js
import init, { check, make, addressHasReceiver } from "./wasm/pkg/zcash_delivery_proof_wasm.js";
await init();
const delivery = JSON.parse(check(txHex, proof, "mainnet"));   // txid, wtxid, pool, action, address, value, memoText
const found = JSON.parse(make(txHex, viewingKey));             // [{ proof, side, pool, action, address, value, memoText }]
```

In Node, `initSync({ module: bytes })` loads it from the file: `node examples/node/check.mjs` checks every
test vector this way. `examples/web/index.html` is a page that checks a pasted proof. A viewing key given
to `make` stays in memory; the package does no I/O.

## Test vectors

- `test-vectors/constructed.json`: one Orchard (v5) and one Ironwood (v6) transaction. Each pays a
  merchant and a second party from a transparent coin, with the keys that see each payment (the
  merchant's UFVK and UIVK, the other party's, the sender's, and a stranger's) and every proof.
  - They're built by librustzcash's own transaction builder, under testnet rules, and never broadcast.
  - Every key comes from a label under `sapling.cash/zdp/` (`tests/common`).
  - `tests/vectors.rs` rebuilds the file and requires it to match.
- `test-vectors/testnet.json`: a real transaction mined on Zcash testnet that pays an Ironwood note with a
  memo, the test wallet's UFVK, and the proof.

## Build and test

```sh
cargo test --locked
cargo test --locked --features verify-bundle
node examples/node/check.mjs
```

`rust-toolchain.toml` pins the compiler and `Cargo.lock` the crates. CI runs these on every push, with a
secret scan of the full history and the WebAssembly check below.

### Reproducible WebAssembly

`wasm/build.sh` rebuilds `wasm/pkg` byte for byte from pinned inputs:
- the Rust toolchain and `wasm/Cargo.lock`;
- Ubuntu 26.04's clang 21.1.8 (secp256k1's C code is compiled in, through zcash_primitives);
- the official wasm-bindgen 0.2.129 release binary, checked by its SHA-256;
- no wasm-opt pass;
- a fixed build directory with remapped paths.

`wasm/build.sh --check` fails if the committed files differ; `wasm/SHA256` holds the module's hash. The
module carries a custom section named `sapling.cash` that says where it comes from.

## Status

Version 0.1.0, published on GitHub only. It is not yet on crates.io or npm, and not yet independently
reviewed.

## License

Apache License 2.0; see [LICENSE](LICENSE), [NOTICE](NOTICE) (the attribution redistributions must keep)
and [TRADEMARKS.md](TRADEMARKS.md).
