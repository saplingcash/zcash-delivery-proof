//! zcash-delivery-proof as WebAssembly, for a browser or Node. Nothing here does any I/O: a viewing key
//! given to `make` stays in the caller's memory.

use serde::Serialize;
use wasm_bindgen::prelude::*;
use zcash_delivery_proof::{memo_text, pool_receiver_address, DeliveryProof, Pool, Side, ViewingKeys};
use zcash_protocol::consensus::NetworkType;

/// Where this module comes from, in a custom section of the WebAssembly binary named "sapling.cash"
/// (`wasm-objdump -j sapling.cash -s` shows it). Plain text; nothing reads it at run time.
#[used]
#[link_section = "sapling.cash"]
static ORIGIN: [u8; 108] = *b"zcash-delivery-proof, from https://sapling.cash; source: https://github.com/saplingcash/zcash-delivery-proof";

fn tx_bytes(tx_hex: &str) -> Result<Vec<u8>, JsError> {
    hex::decode(tx_hex.trim()).map_err(|_| JsError::new("the transaction must be hex"))
}

fn network(name: &str) -> Result<NetworkType, JsError> {
    match name {
        "mainnet" => Ok(NetworkType::Main),
        "testnet" => Ok(NetworkType::Test),
        "regtest" => Ok(NetworkType::Regtest),
        _ => Err(JsError::new("the network is mainnet, testnet or regtest")),
    }
}

fn err(e: zcash_delivery_proof::Error) -> JsError {
    JsError::new(&e.to_string())
}

/// Checks a proof (`zdp:1:…`) against the transaction's bytes (hex) with no key. Returns the delivery as
/// JSON: txid, wtxid, pool, action, address (in `network`'s encoding), value, memoHex, memoText.
#[wasm_bindgen]
pub fn check(tx_hex: &str, proof: &str, network_name: &str) -> Result<String, JsError> {
    let net = network(network_name)?;
    let d = zcash_delivery_proof::check(&tx_bytes(tx_hex)?, &DeliveryProof::decode(proof).map_err(err)?).map_err(err)?;
    serde_json::to_string(&d.view(net)).map_err(|e| JsError::new(&e.to_string()))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FoundView {
    proof: String,
    side: Side,
    pool: Pool,
    action: u16,
    address: String,
    value: u64,
    memo_text: Option<String>,
    memo_hex: String,
}

fn found_json(found: &[zcash_delivery_proof::Found], network: NetworkType) -> Result<String, JsError> {
    let views: Vec<FoundView> = found
        .iter()
        .map(|f| FoundView {
            proof: f.proof.encode(),
            side: f.side,
            pool: f.proof.pool,
            action: f.proof.action,
            address: pool_receiver_address(f.proof.pool, &f.proof.receiver, network),
            value: f.proof.value,
            memo_text: memo_text(&f.memo),
            memo_hex: hex::encode(f.memo),
        })
        .collect();
    serde_json::to_string(&views).map_err(|e| JsError::new(&e.to_string()))
}

/// Every note a UFVK (received and sent) or UIVK (received) sees in the transaction (Orchard, Ironwood and
/// Sapling), each with its proof, as a JSON array. The key's own network chooses the address encoding.
#[wasm_bindgen]
pub fn make(tx_hex: &str, viewing_key: &str) -> Result<String, JsError> {
    let keys = ViewingKeys::parse(viewing_key).map_err(err)?;
    found_json(&zcash_delivery_proof::make(&tx_bytes(tx_hex)?, &keys).map_err(err)?, keys.network)
}

fn ovks(list: &str) -> Result<Vec<[u8; 32]>, JsError> {
    list.split(',').map(str::trim).filter(|s| !s.is_empty()).map(|h| hex::decode(h).ok().and_then(|v| v.try_into().ok()).ok_or_else(|| JsError::new("an outgoing viewing key is 32 bytes of hex"))).collect()
}

/// Every note the given outgoing viewing keys sent in the transaction, each with its proof, as a JSON
/// array (as `make`). `orchardOvks` and `saplingOvks`: comma-separated, 32 bytes of hex each, either may
/// be empty. For a sender that publishes its outgoing viewing key, so that anyone can prove what it sent.
#[wasm_bindgen(js_name = makeWithOutgoingKeys)]
pub fn make_with_outgoing_keys(tx_hex: &str, network_name: &str, orchard_ovks: &str, sapling_ovks: &str) -> Result<String, JsError> {
    let net = network(network_name)?;
    let keys = ViewingKeys::from_outgoing_keys(net, &ovks(orchard_ovks)?, &ovks(sapling_ovks)?);
    found_json(&zcash_delivery_proof::make(&tx_bytes(tx_hex)?, &keys).map_err(err)?, net)
}

/// Whether an address (a unified address as a payer was given it, or a Sapling address) carries the
/// receiver a proof names, in the proof's pool.
#[wasm_bindgen(js_name = addressHasReceiver)]
pub fn address_has_receiver(address: &str, proof: &str) -> Result<bool, JsError> {
    let p = DeliveryProof::decode(proof).map_err(err)?;
    Ok(zcash_delivery_proof::address_has_pool_receiver(address, p.pool, &p.receiver))
}
