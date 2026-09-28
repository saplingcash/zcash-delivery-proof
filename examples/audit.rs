//! An audit: an organisation shows an auditor the incoming payments it chooses, one proof each, without
//! giving the auditor a viewing key (which would reveal every payment, past and future).
//!
//!   cargo run --example audit
use zcash_delivery_proof::{check, make, memo_text, DeliveryProof, Side, ViewingKeys};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/test-vectors/constructed.json"))?)?;
    // the organisation: its full viewing keys (here one per test transaction), over the transactions under audit
    let mut disclosed: Vec<(Vec<u8>, String)> = Vec::new();
    for case in v["cases"].as_array().unwrap() {
        let tx = hex::decode(case["txHex"].as_str().unwrap())?;
        let org = ViewingKeys::parse(case["keys"]["merchantUfvk"].as_str().unwrap())?;
        for f in make(&tx, &org)?.into_iter().filter(|f| f.side == Side::Received) {
            disclosed.push((tx.clone(), f.proof.encode()));
        }
    }
    println!("the organisation discloses {} payments\n", disclosed.len());

    // the auditor: each proof against its transaction, then the total
    let mut total = 0u64;
    for (tx, proof) in &disclosed {
        let d = check(tx, &DeliveryProof::decode(proof)?)?;
        println!("{:?} {} zatoshi in {}: \"{}\"", d.pool, d.value, d.txid_hex(), memo_text(&d.memo).unwrap_or_default());
        total += d.value;
    }
    println!("\nproved received: {total} zatoshi");
    Ok(())
}
