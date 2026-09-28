//! A proof holds only for exactly the note it names: change any part of it, or the transaction, and the
//! check fails. Run on both constructed transactions (Orchard and Ironwood).

mod common;

use serde_json::Value;
use zcash_delivery_proof::{check, DeliveryProof, Error, Pool};

fn cases() -> Vec<(Vec<u8>, DeliveryProof, DeliveryProof)> {
    let v: Value = serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/test-vectors/constructed.json")).unwrap()).unwrap();
    v["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            let tx = hex::decode(c["txHex"].as_str().unwrap()).unwrap();
            let p = |i: usize| DeliveryProof::decode(c["payments"][i]["proof"].as_str().unwrap()).unwrap();
            (tx, p(0), p(1))
        })
        .collect()
}

fn mismatch(r: Result<zcash_delivery_proof::Delivery, Error>) -> String {
    match r {
        Err(Error::Mismatch(m)) => m,
        other => panic!("expected a mismatch, got {other:?}"),
    }
}

#[test]
fn every_field_of_the_proof_matters() {
    for (tx, proof, other) in cases() {
        assert!(check(&tx, &proof).is_ok());
        let with = |f: &dyn Fn(&mut DeliveryProof)| {
            let mut p = proof.clone();
            f(&mut p);
            check(&tx, &p)
        };
        assert!(mismatch(with(&|p| p.txid[0] ^= 1)).contains("txids differ"));
        assert!(mismatch(with(&|p| p.pool = if p.pool == Pool::Orchard { Pool::Ironwood } else { Pool::Orchard })).contains("bundle"));
        assert!(mismatch(with(&|p| p.action = 7)).contains("no action 7"));
        // the other action of the same bundle: another note entirely
        assert!(mismatch(with(&|p| p.action = other.action)).contains("commit"));
        // another receiver, a value one zatoshi off, another rseed
        assert!(mismatch(with(&|p| p.receiver = other.receiver)).contains("commit"));
        assert!(mismatch(with(&|p| p.value += 1)).contains("commit"));
        assert!(mismatch(with(&|p| p.rseed = common::label("tests/wrong-rseed"))).len() > 0);
        // the other payment's proof checks, for its own note
        assert!(check(&tx, &other).is_ok());
    }
}

#[test]
fn a_changed_ciphertext_does_not_decrypt_even_under_its_new_txid() {
    for (tx, proof, _) in cases() {
        let t = zcash_delivery_proof::read_tx(&tx).unwrap();
        let b = match proof.pool {
            Pool::Orchard => t.orchard_bundle(),
            Pool::Ironwood => t.ironwood_bundle(),
        }
        .unwrap();
        let action = &b.actions()[usize::from(proof.action)];
        // find the action's encrypted note (with its memo) in the bytes and change one byte of it
        let enc = &action.encrypted_note().enc_ciphertext;
        let at = tx.windows(enc.len()).position(|w| w == &enc[..]).expect("the ciphertext is in the bytes") + 100;
        let mut forged = tx.clone();
        forged[at] ^= 0x01;
        // the forger also points the proof at the new transaction's txid
        let ft = zcash_delivery_proof::read_tx(&forged).unwrap();
        let mut p = proof.clone();
        p.txid = *ft.txid().as_ref();
        assert!(mismatch(check(&forged, &p)).contains("does not decrypt"), "the memo is authenticated by the note encryption");
        // and the original proof no longer names it
        assert!(mismatch(check(&forged, &proof)).contains("txids differ"));
    }
}

#[test]
fn malformed_input_is_refused_with_a_reason() {
    let (tx, proof, _) = cases().remove(0);
    assert!(matches!(DeliveryProof::decode("splg-proof:1:AAAA"), Err(Error::Proof(_))));
    assert!(matches!(DeliveryProof::decode("zdp:1:not base64!"), Err(Error::Proof(_))));
    assert!(matches!(DeliveryProof::decode("zdp:1:AAAA"), Err(Error::Proof(_))));
    let mut b = proof.to_bytes();
    b[32] = 9;
    assert!(matches!(DeliveryProof::from_bytes(&b), Err(Error::Proof(_))));
    assert!(matches!(check(&tx[..tx.len() - 1], &proof), Err(Error::Transaction(_))));
    assert!(matches!(check(&[], &proof), Err(Error::Transaction(_))));
    let mut extra = tx.clone();
    extra.push(0);
    assert!(matches!(check(&extra, &proof), Err(Error::Transaction(_))));
    assert!(matches!(zcash_delivery_proof::ViewingKeys::parse("uview1notakey"), Err(Error::Key(_))));
}
