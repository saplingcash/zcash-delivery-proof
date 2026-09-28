//! A real transaction mined on Zcash mainnet (test-vectors/mainnet.json): its proof checks with no key, to
//! the value and memo the payment carried, and a key that is not the receiver's finds nothing.

mod common;

use serde_json::Value;
use zcash_delivery_proof::{check, make, memo_text, DeliveryProof, Pool, ViewingKeys};
use zcash_protocol::consensus::NetworkType;

fn vector() -> Value {
    serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/test-vectors/mainnet.json")).unwrap()).unwrap()
}

#[test]
fn the_proof_checks_with_no_key() {
    let v = vector();
    let tx = hex::decode(v["txHex"].as_str().unwrap()).unwrap();
    let proof = DeliveryProof::decode(v["proof"].as_str().unwrap()).unwrap();
    let d = check(&tx, &proof).unwrap();
    assert_eq!(d.pool, Pool::Ironwood);
    assert_eq!(d.txid_hex(), v["txid"].as_str().unwrap());
    assert_eq!(d.value, v["value"].as_u64().unwrap());
    assert_eq!(memo_text(&d.memo).as_deref(), v["memoText"].as_str());
    assert_eq!(d.view(NetworkType::Main).address, v["address"].as_str().unwrap());
}

#[test]
fn another_key_finds_nothing() {
    let v = vector();
    let tx = hex::decode(v["txHex"].as_str().unwrap()).unwrap();
    assert!(make(&tx, &ViewingKeys::parse(&common::ufvk("tests/not-the-wallet")).unwrap()).unwrap().is_empty());
}

#[cfg(feature = "verify-bundle")]
#[test]
fn the_bundle_verifies() {
    // fully shielded: no transparent inputs, so no spent coins are needed
    let v = vector();
    let tx = hex::decode(v["txHex"].as_str().unwrap()).unwrap();
    zcash_delivery_proof::verify_bundle(&tx, Pool::Ironwood, &[]).unwrap();
}
