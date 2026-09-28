//! A receipt: a shop proves it received a payment, with its memo, to anyone it chooses (a customer, an
//! accountant), using only its incoming viewing key and without handing that key over.
//!
//!   cargo run --example receipt
use zcash_delivery_proof::{check, make, memo_text, receiver_address, DeliveryProof, ViewingKeys};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // the shop's side: its UIVK and the transaction it was paid in (here, from the test vectors)
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/test-vectors/constructed.json"))?)?;
    let case = &v["cases"][1];
    let tx = hex::decode(case["txHex"].as_str().unwrap())?;
    let shop = ViewingKeys::parse(case["keys"]["merchantUivk"].as_str().unwrap())?;
    let found = make(&tx, &shop)?;
    let receipt = found.first().ok_or("the shop's key finds no payment in this transaction")?.proof.encode();
    println!("the shop hands over: {receipt}\n");

    // anyone's side: the proof and the transaction's bytes (from any node or explorer), no key
    let d = check(&tx, &DeliveryProof::decode(&receipt)?)?;
    println!("transaction {} delivered {} zatoshi", d.txid_hex(), d.value);
    println!("to {}", receiver_address(&d.receiver, shop.network));
    println!("with the memo: {}", memo_text(&d.memo).unwrap_or_default());
    println!("(to be sure the transaction is mined, look its txid up on a node or explorer)");
    Ok(())
}
