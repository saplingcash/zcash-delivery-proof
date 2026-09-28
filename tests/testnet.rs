//! A real transaction mined on Zcash testnet (test-vectors/testnet.json): the wallet's viewing key finds its
//! Ironwood note, the proof checks with no key, and a key that is not the wallet's finds nothing.

mod common;

use serde_json::Value;
use zcash_delivery_proof::{check, make, memo_text, DeliveryProof, Pool, Side, ViewingKeys};

fn vector() -> Value {
    serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/test-vectors/testnet.json")).unwrap()).unwrap()
}

#[test]
fn the_wallet_finds_its_note_and_the_proof_checks() {
    let v = vector();
    let tx = hex::decode(v["txHex"].as_str().unwrap()).unwrap();
    let found = make(&tx, &ViewingKeys::parse(v["ufvk"].as_str().unwrap()).unwrap()).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].side, Side::Received);
    assert_eq!(found[0].proof.encode(), v["proof"].as_str().unwrap());
    let d = check(&tx, &DeliveryProof::decode(v["proof"].as_str().unwrap()).unwrap()).unwrap();
    assert_eq!((d.pool, d.value, d.txid_hex().as_str()), (Pool::Ironwood, 546, v["txid"].as_str().unwrap()));
    assert!(memo_text(&d.memo).is_some_and(|t| !t.is_empty()));
}

#[test]
fn another_key_finds_nothing() {
    let v = vector();
    let tx = hex::decode(v["txHex"].as_str().unwrap()).unwrap();
    assert!(make(&tx, &ViewingKeys::parse(&common::ufvk("tests/not-the-wallet")).unwrap()).unwrap().is_empty());
}
