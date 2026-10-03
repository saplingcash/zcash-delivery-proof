//! Real transactions mined on Zcash testnet: test-vectors/testnet.json (an Ironwood note) and
//! test-vectors/testnet-sapling.json (a Sapling note). The wallet's viewing key finds its note (and, for the
//! Sapling one, the sender's outgoing viewing key too), the proof checks with no key, and a key that is not
//! the wallet's finds nothing.

mod common;

use serde_json::Value;
use zcash_delivery_proof::{check, make, memo_text, DeliveryProof, Pool, Side, ViewingKeys};
use zcash_protocol::consensus::NetworkType;

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

// ---- a real Sapling payment on Zcash testnet (test-vectors/testnet-sapling.json)

fn sapling_vector() -> Value {
    serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/test-vectors/testnet-sapling.json")).unwrap()).unwrap()
}

#[test]
fn a_sapling_note_is_found_by_the_wallet_and_by_the_sender_and_checks_with_no_key() {
    let v = sapling_vector();
    let tx = hex::decode(v["txHex"].as_str().unwrap()).unwrap();
    // the keys are the labels' (tests/common)
    assert_eq!(v["ufvk"].as_str().unwrap(), common::sapling_ufvk("testnet-sapling/wallet"));
    let ovk = common::sapling_dfvk("testnet-sapling/sender").to_ovk(zip32::Scope::External).0;
    assert_eq!(v["senderOvk"].as_str().unwrap(), hex::encode(ovk));
    // the wallet: received
    let received = make(&tx, &ViewingKeys::parse(v["ufvk"].as_str().unwrap()).unwrap()).unwrap();
    assert_eq!(received.len(), 1);
    assert_eq!((received[0].side, received[0].proof.encode().as_str()), (Side::Received, v["proof"].as_str().unwrap()));
    // the sender's outgoing viewing key alone: sent, the same proof
    let sent = make(&tx, &ViewingKeys::from_outgoing_keys(NetworkType::Test, &[], &[ovk])).unwrap();
    assert_eq!(sent.len(), 1);
    assert_eq!((sent[0].side, sent[0].proof.encode().as_str()), (Side::Sent, v["proof"].as_str().unwrap()));
    // anyone: the proof, against the bytes, no key
    let d = check(&tx, &DeliveryProof::decode(v["proof"].as_str().unwrap()).unwrap()).unwrap();
    assert_eq!((d.pool, d.value, d.txid_hex().as_str()), (Pool::Sapling, v["value"].as_u64().unwrap(), v["txid"].as_str().unwrap()));
    assert_eq!(memo_text(&d.memo).as_deref(), v["memoText"].as_str());
    assert_eq!(d.view(NetworkType::Test).address, v["address"].as_str().unwrap());
    assert!(zcash_delivery_proof::address_has_pool_receiver(v["address"].as_str().unwrap(), Pool::Sapling, &d.receiver));
    // the transaction's one input spends an output of the transaction given with it
    let t = zcash_delivery_proof::read_tx(&tx).unwrap();
    let prev = zcash_delivery_proof::read_tx(&hex::decode(v["spentTxHex"].as_str().unwrap()).unwrap()).unwrap();
    assert_eq!(prev.txid().as_ref(), t.transparent_bundle().unwrap().vin[0].prevout().hash());
}

#[test]
fn a_key_that_is_not_the_sapling_wallets_finds_nothing() {
    let v = sapling_vector();
    let tx = hex::decode(v["txHex"].as_str().unwrap()).unwrap();
    for key in [common::sapling_ufvk("tests/not-the-wallet"), common::ufvk("testnet-sapling/wallet")] {
        assert!(make(&tx, &ViewingKeys::parse(&key).unwrap()).unwrap().is_empty());
    }
    assert!(make(&tx, &ViewingKeys::from_outgoing_keys(NetworkType::Test, &[], &[[1u8; 32]])).unwrap().is_empty());
}
