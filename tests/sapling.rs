//! Sapling notes (the constructed Sapling transaction in test-vectors/constructed.json): the keys that
//! find them, the addresses a proof names, and the checks that keep the pools apart.

mod common;

use serde_json::Value;
use zcash_address::unified::{self, Encoding};
use zcash_address::{ToAddress, ZcashAddress};
use zcash_delivery_proof::{address_has_pool_receiver, check, make, pool_receiver_address, DeliveryProof, Error, Pool, Side, ViewingKeys};
use zcash_protocol::consensus::NetworkType;

fn case(pool: &str) -> Value {
    let v: Value = serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/test-vectors/constructed.json")).unwrap()).unwrap();
    v["cases"].as_array().unwrap().iter().find(|c| c["pool"] == pool).unwrap().clone()
}

fn tx(c: &Value) -> Vec<u8> {
    hex::decode(c["txHex"].as_str().unwrap()).unwrap()
}

fn proof(c: &Value, i: usize) -> DeliveryProof {
    DeliveryProof::decode(c["payments"][i]["proof"].as_str().unwrap()).unwrap()
}

#[test]
fn a_ufvk_with_both_items_sees_both_protocols() {
    // one wallet with an Orchard and a Sapling item: its Sapling note in the Sapling transaction
    let name = "vectors/sapling/merchant";
    let both = unified::Ufvk::try_from_items(unified::Revision::R0, vec![unified::Uitem::Data(unified::Fvk::Orchard(common::fvk(name).to_bytes())), unified::Uitem::Data(unified::Fvk::Sapling(common::sapling_dfvk(name).to_bytes()))]).unwrap().encode(&NetworkType::Test);
    let keys = ViewingKeys::parse(&both).unwrap();
    assert_eq!((keys.incoming.len(), keys.outgoing.len(), keys.sapling_incoming.len(), keys.sapling_outgoing.len()), (2, 2, 2, 2));
    let c = case("sapling");
    let f = make(&tx(&c), &keys).unwrap();
    assert_eq!(f.len(), 1);
    assert_eq!((f[0].side, f[0].proof.pool, f[0].proof.encode()), (Side::Received, Pool::Sapling, proof(&c, 0).encode()));
    // the Orchard-only key of the same label finds nothing in it
    assert!(make(&tx(&c), &ViewingKeys::parse(&common::ufvk(name)).unwrap()).unwrap().is_empty());
}

#[test]
fn a_published_outgoing_key_alone_finds_what_was_sent_with_it() {
    let c = case("sapling");
    let ovk = common::sapling_dfvk("vectors/sapling/sender").to_ovk(zip32::Scope::External).0;
    let keys = ViewingKeys::from_outgoing_keys(NetworkType::Test, &[], &[ovk]);
    let f = make(&tx(&c), &keys).unwrap();
    assert_eq!(f.len(), 2);
    assert!(f.iter().all(|x| x.side == Side::Sent && x.proof.pool == Pool::Sapling));
    for (i, x) in f.iter().enumerate() {
        assert_eq!(check(&tx(&c), &x.proof).unwrap().value, x.proof.value, "payment {i}");
    }
    // the same 32 bytes as an Orchard key see nothing in a Sapling bundle, and another key nothing at all
    assert!(make(&tx(&c), &ViewingKeys::from_outgoing_keys(NetworkType::Test, &[ovk], &[])).unwrap().is_empty());
    assert!(make(&tx(&c), &ViewingKeys::from_outgoing_keys(NetworkType::Test, &[], &[[7u8; 32]])).unwrap().is_empty());
}

#[test]
fn a_sapling_uivk_sees_received_notes_only() {
    let c = case("sapling");
    let f = make(&tx(&c), &ViewingKeys::parse(&common::sapling_uivk("vectors/sapling/merchant")).unwrap()).unwrap();
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].side, Side::Received);
    // the sender's UIVK received nothing
    assert!(make(&tx(&c), &ViewingKeys::parse(&common::sapling_uivk("vectors/sapling/sender")).unwrap()).unwrap().is_empty());
}

#[test]
fn the_receiver_is_a_sapling_address() {
    let c = case("sapling");
    let p = proof(&c, 0);
    let ua = pool_receiver_address(Pool::Sapling, &p.receiver, NetworkType::Test);
    // a unified address with the one Sapling receiver
    let (net, _revision, decoded) = unified::Address::decode(&ua).unwrap();
    assert_eq!(net, NetworkType::Test);
    assert_eq!(unified::Container::items(&decoded), vec![unified::Receiver::Sapling(p.receiver)]);
    assert!(address_has_pool_receiver(&ua, Pool::Sapling, &p.receiver));
    // the plain Sapling address (ztestsapling…) of the same receiver matches too
    let plain = ZcashAddress::from_sapling(NetworkType::Test, p.receiver).encode();
    assert!(plain.starts_with("ztestsapling1"));
    assert!(address_has_pool_receiver(&plain, Pool::Sapling, &p.receiver));
    // the same bytes are not an Orchard receiver of that address, and another receiver does not match
    assert!(!address_has_pool_receiver(&ua, Pool::Orchard, &p.receiver));
    assert!(!zcash_delivery_proof::address_has_receiver(&ua, &p.receiver));
    assert!(!address_has_pool_receiver(&ua, Pool::Sapling, &proof(&c, 1).receiver));
    assert!(!address_has_pool_receiver("not an address", Pool::Sapling, &p.receiver));
}

#[test]
fn the_pool_is_part_of_the_proof() {
    // a Sapling proof named as Ironwood, and an Ironwood proof named as Sapling, do not check
    let s = case("sapling");
    let mut p = proof(&s, 0);
    p.pool = Pool::Ironwood;
    assert!(matches!(check(&tx(&s), &p), Err(Error::Mismatch(m)) if m.contains("no Ironwood bundle")));
    let i = case("ironwood");
    let mut p = proof(&i, 0);
    p.pool = Pool::Sapling;
    assert!(matches!(check(&tx(&i), &p), Err(Error::Mismatch(m)) if m.contains("no Sapling outputs")));
    // pool 3 is Sapling in the bytes; 4 is refused
    let mut b = proof(&s, 0).to_bytes();
    assert_eq!(b[32], 3);
    b[32] = 4;
    assert!(matches!(DeliveryProof::from_bytes(&b), Err(Error::Proof(_))));
}

#[test]
fn a_receiver_that_is_not_a_sapling_address_is_refused() {
    let s = case("sapling");
    let mut p = proof(&s, 0);
    // a diversifier with no valid base point (found by search over label-derived bytes)
    let bad = (0u32..)
        .map(|i| common::label::<43>(&format!("tests/bad-diversifier#{i}")))
        .find(|r| sapling::PaymentAddress::from_bytes(r).is_none())
        .unwrap();
    p.receiver = bad;
    assert!(matches!(check(&tx(&s), &p), Err(Error::Mismatch(m)) if m.contains("not a valid Sapling payment address")));
}
