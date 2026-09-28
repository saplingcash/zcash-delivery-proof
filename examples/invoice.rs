//! An invoice: a customer paid invoice 1042 and the merchant says nothing arrived. The customer proves
//! the payment from its own wallet (its outgoing viewing key saw the note when it was sent), and an
//! arbiter checks it against the address and amount on the invoice.
//!
//!   cargo run --example invoice
use zcash_delivery_proof::{address_has_receiver, check, make, memo_text, DeliveryProof, Side, ViewingKeys};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/test-vectors/constructed.json"))?)?;
    let case = &v["cases"][0];
    let tx = hex::decode(case["txHex"].as_str().unwrap())?;
    // what the invoice said: pay 0.00123456 ZEC to this address, quoting "Invoice 1042"
    let invoice_address = case["payments"][0]["to"].as_str().unwrap();
    let invoice_amount = 123_456u64;

    // the customer: its UFVK finds the notes it sent in the payment transaction
    let customer = ViewingKeys::parse(case["keys"]["senderUfvk"].as_str().unwrap())?;
    let paid = make(&tx, &customer)?
        .into_iter()
        .filter(|f| f.side == Side::Sent)
        .find(|f| address_has_receiver(invoice_address, &f.proof.receiver))
        .ok_or("the customer sent nothing to the invoice's address in this transaction")?;
    let proof = paid.proof.encode();
    println!("the customer shows: {proof}\n");

    // the arbiter: no key, only the proof, the transaction's bytes and the invoice
    let d = check(&tx, &DeliveryProof::decode(&proof)?)?;
    let memo = memo_text(&d.memo).unwrap_or_default();
    assert!(address_has_receiver(invoice_address, &d.receiver), "paid to the invoice's address");
    assert!(d.value >= invoice_amount, "at least the invoiced amount");
    assert!(memo.contains("Invoice 1042"), "quoting the invoice");
    println!("paid: {} zatoshi to the invoice's address, memo \"{memo}\", in transaction {}", d.value, d.txid_hex());
    Ok(())
}
