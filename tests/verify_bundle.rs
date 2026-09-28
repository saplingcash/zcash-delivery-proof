//! The `verify-bundle` feature: the bundles of the constructed and the testnet transactions verify, and a
//! changed ciphertext (which changes the signature hash) does not.
#![cfg(feature = "verify-bundle")]

use serde_json::Value;
use zcash_delivery_proof::{verify_bundle, Pool, SpentCoin};

fn read(file: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(format!("{}/test-vectors/{file}", env!("CARGO_MANIFEST_DIR"))).unwrap()).unwrap()
}

#[test]
fn real_bundles_verify_and_a_changed_one_does_not() {
    let constructed = read("constructed.json");
    for c in constructed["cases"].as_array().unwrap() {
        let tx = hex::decode(c["txHex"].as_str().unwrap()).unwrap();
        let pool = if c["pool"] == "orchard" { Pool::Orchard } else { Pool::Ironwood };
        let spent: Vec<SpentCoin> = c["spent"].as_array().unwrap().iter().map(|s| SpentCoin { value: s["value"].as_u64().unwrap(), script_pubkey: hex::decode(s["scriptPubKey"].as_str().unwrap()).unwrap() }).collect();
        verify_bundle(&tx, pool, &spent).unwrap();
        // a bundle the transaction does not have; the spent coins missing; a wrong spent value
        assert!(verify_bundle(&tx, if pool == Pool::Orchard { Pool::Ironwood } else { Pool::Orchard }, &spent).is_err());
        assert!(verify_bundle(&tx, pool, &[]).is_err());
        let wrong = vec![SpentCoin { value: spent[0].value + 1, ..spent[0].clone() }];
        assert!(verify_bundle(&tx, pool, &wrong).is_err());
        let mut changed = tx.clone();
        let n = changed.len();
        changed[n - 700] ^= 1;
        if zcash_delivery_proof::read_tx(&changed).is_ok() {
            assert!(verify_bundle(&changed, pool, &spent).is_err());
        }
    }
    // the testnet transaction's one input spends an output of the transaction given with it
    let t = read("testnet.json");
    let tx = hex::decode(t["txHex"].as_str().unwrap()).unwrap();
    let parsed = zcash_delivery_proof::read_tx(&tx).unwrap();
    let prevout = parsed.transparent_bundle().unwrap().vin[0].prevout().clone();
    let prev = zcash_delivery_proof::read_tx(&hex::decode(t["spentTxHex"].as_str().unwrap()).unwrap()).unwrap();
    assert_eq!(prev.txid().as_ref(), prevout.hash());
    let out = &prev.transparent_bundle().unwrap().vout[prevout.n() as usize];
    let script: &zcash_transparent::address::Script = &out.script_pubkey().clone().into();
    verify_bundle(&tx, Pool::Ironwood, &[SpentCoin { value: u64::from(out.value()), script_pubkey: script.0 .0.clone() }]).unwrap();
}
