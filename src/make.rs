//! Making proofs: every note a viewing key can see in a transaction.
use sapling::note_encryption::{try_sapling_note_decryption, try_sapling_output_recovery, Zip212Enforcement};
use serde::Serialize;

use crate::tx::{bundle, read_tx, sapling_outputs};
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

/// Every note in `tx` (Orchard, Ironwood and Sapling) that `keys` received or sent, with its proof. A
/// note found both ways (a payment to oneself) is listed once, as received. The proof is the same either
/// way: it is a fact about the note, not about who made it.
pub fn make(tx: &[u8], keys: &ViewingKeys) -> Result<Vec<Found>, Error> {
    let t = read_tx(tx)?;
    let txid = *t.txid().as_ref();
    let mut found: Vec<Found> = Vec::new();
    let mut add = |pool: Pool, idx: usize, side: Side, receiver: [u8; 43], value: u64, rseed: [u8; 32], memo: [u8; 512]| -> Result<(), Error> {
        if found.iter().any(|f| f.proof.pool == pool && usize::from(f.proof.action) == idx) {
            return Ok(());
        }
        let action = u16::try_from(idx).map_err(|_| Error::Transaction("a bundle with more than 65,535 actions".into()))?;
        found.push(Found { proof: DeliveryProof { txid, pool, action, receiver, value, rseed }, side, memo });
        Ok(())
    };
    for pool in [Pool::Orchard, Pool::Ironwood] {
        let Some(b) = bundle(&t, pool) else { continue };
        for (idx, _ivk, note, to, memo) in b.decrypt_outputs_with_keys(&keys.incoming) {
            add(pool, idx, Side::Received, to.to_raw_address_bytes(), note.value().inner(), *note.rseed().as_bytes(), memo)?;
        }
        for (idx, _ovk, note, to, memo) in b.recover_outputs_with_ovks(&keys.outgoing) {
            add(pool, idx, Side::Sent, to.to_raw_address_bytes(), note.value().inner(), *note.rseed().as_bytes(), memo)?;
        }
    }
    // Sapling: only notes with plaintext version 2 (ZIP 212), whose rseed gives their esk, can be proved
    let after_zip212 = |note: &sapling::Note| match note.rseed() {
        sapling::Rseed::AfterZip212(r) => Some(*r),
        sapling::Rseed::BeforeZip212(_) => None,
    };
    if !keys.sapling_incoming.is_empty() || !keys.sapling_outgoing.is_empty() {
        for (idx, output) in sapling_outputs(&t).iter().enumerate() {
            for ivk in &keys.sapling_incoming {
                if let Some((note, to, memo)) = try_sapling_note_decryption(ivk, output, Zip212Enforcement::On) {
                    if let Some(rseed) = after_zip212(&note) {
                        add(Pool::Sapling, idx, Side::Received, to.to_bytes(), note.value().inner(), rseed, memo)?;
                    }
                }
            }
            for ovk in &keys.sapling_outgoing {
                if let Some((note, to, memo)) = try_sapling_output_recovery(ovk, output, Zip212Enforcement::On) {
                    if let Some(rseed) = after_zip212(&note) {
                        add(Pool::Sapling, idx, Side::Sent, to.to_bytes(), note.value().inner(), rseed, memo)?;
                    }
                }
            }
        }
    }
    found.sort_by_key(|f| (f.proof.pool.to_byte(), f.proof.action));
    Ok(found)
}
