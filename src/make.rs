//! Making proofs: every note a viewing key can see in a transaction.
use serde::Serialize;

use crate::tx::{bundle, read_tx};
use crate::{DeliveryProof, Error, Pool, ViewingKeys};

/// Which side of the payment the key is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    /// found with an incoming viewing key: the key's wallet received the note
    Received,
    /// found with an outgoing viewing key: the key's wallet sent the note (and set its outgoing viewing
    /// key when it did, as wallets do by default)
    Sent,
}

/// A note the key can see, and the proof of its delivery.
#[derive(Clone, Debug)]
pub struct Found {
    pub proof: DeliveryProof,
    pub side: Side,
    pub memo: [u8; 512],
}

/// Every note in `tx` (Orchard and Ironwood) that `keys` received or sent, with its proof. A note found
/// both ways (a payment to oneself) is listed once, as received. The proof is the same either way: it
/// is a fact about the note, not about who made it.
pub fn make(tx: &[u8], keys: &ViewingKeys) -> Result<Vec<Found>, Error> {
    let t = read_tx(tx)?;
    let txid = *t.txid().as_ref();
    let mut found: Vec<Found> = Vec::new();
    for pool in [Pool::Orchard, Pool::Ironwood] {
        let Some(b) = bundle(&t, pool) else { continue };
        let mut add = |idx: usize, side: Side, note: orchard::Note, to: orchard::Address, memo: [u8; 512]| -> Result<(), Error> {
            if found.iter().any(|f| f.proof.pool == pool && usize::from(f.proof.action) == idx) {
                return Ok(());
            }
            let action = u16::try_from(idx).map_err(|_| Error::Transaction("a bundle with more than 65,535 actions".into()))?;
            let proof = DeliveryProof { txid, pool, action, receiver: to.to_raw_address_bytes(), value: note.value().inner(), rseed: *note.rseed().as_bytes() };
            found.push(Found { proof, side, memo });
            Ok(())
        };
        for (idx, _ivk, note, to, memo) in b.decrypt_outputs_with_keys(&keys.incoming) {
            add(idx, Side::Received, note, to, memo)?;
        }
        for (idx, _ovk, note, to, memo) in b.recover_outputs_with_ovks(&keys.outgoing) {
            add(idx, Side::Sent, note, to, memo)?;
        }
    }
    found.sort_by_key(|f| (f.proof.pool.to_byte(), f.proof.action));
    Ok(found)
}
