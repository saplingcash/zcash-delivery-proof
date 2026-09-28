//! Checking a proof: no key needed.
use orchard::note::{ExtractedNoteCommitment, RandomSeed, Rho};
use orchard::note_encryption::{IronwoodDomain, OrchardDomain};
use orchard::value::NoteValue;
use orchard::{Address, Note};
use serde::Serialize;
use zcash_note_encryption::{try_output_recovery_with_pkd_esk, Domain};
use zcash_protocol::consensus::NetworkType;

use crate::tx::{bundle, read_tx, wtxid};
use crate::{DeliveryProof, Error, Pool};

/// What a proof shows, once checked against the transaction's bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Delivery {
    /// internal byte order
    pub txid: [u8; 32],
    /// ZIP 239, internal byte order: covers the signatures and proofs too
    pub wtxid: [u8; 64],
    pub pool: Pool,
    pub action: u16,
    pub receiver: [u8; 43],
    pub value: u64,
    /// the memo, as the note's authenticated encryption carried it
    pub memo: [u8; 512],
}

/// Checks that the transaction `tx` is the one `proof` names and that its action delivers exactly the
/// note the proof describes:
///
/// 1. the bytes are a transaction that librustzcash reads back to the same bytes, and its txid is the
///    proof's;
/// 2. the named pool has a bundle with the named action;
/// 3. the note rebuilt from the receiver, the value, the action's rho and the rseed (with the pool's
///    note version) has the action's note commitment;
/// 4. the ephemeral key derived from that note (ZIP 212) decrypts the action's ciphertext, with the
///    receiver's transmission key, to exactly that note: the same receiver, value and rseed. The memo
///    comes out of the same authenticated encryption, so it is the memo the sender wrote.
///
/// It does not show that the transaction is mined (compare the txid or wtxid with a node), who sent it,
/// or that the bundle's zk-SNARK and signatures are valid (a mined transaction's are; see the
/// `verify-bundle` feature for an offline check).
pub fn check(tx: &[u8], proof: &DeliveryProof) -> Result<Delivery, Error> {
    let t = read_tx(tx)?;
    if t.txid().as_ref() != &proof.txid {
        return Err(Error::Mismatch("this is not the transaction the proof names (the txids differ)".into()));
    }
    let b = bundle(&t, proof.pool).ok_or_else(|| Error::Mismatch(format!("the transaction has no {:?} bundle", proof.pool)))?;
    let action = b.actions().get(usize::from(proof.action)).ok_or_else(|| Error::Mismatch(format!("the {:?} bundle has no action {}", proof.pool, proof.action)))?;
    let rho = Option::<Rho>::from(Rho::from_bytes(&action.nullifier().to_bytes())).ok_or_else(|| Error::Mismatch("the action's nullifier is not a valid rho".into()))?;
    let rseed = Option::<RandomSeed>::from(RandomSeed::from_bytes(proof.rseed, &rho)).ok_or_else(|| Error::Mismatch("the rseed gives no valid note for this action".into()))?;
    let recipient = Option::<Address>::from(Address::from_raw_address_bytes(&proof.receiver)).ok_or_else(|| Error::Mismatch("the receiver is not a valid Orchard-family address".into()))?;
    let note = Option::<Note>::from(Note::from_parts(recipient, NoteValue::from_raw(proof.value), rho, rseed, b.bundle_version().note_version()))
        .ok_or_else(|| Error::Mismatch("the receiver, value and rseed make no valid note".into()))?;
    if ExtractedNoteCommitment::from(note.commitment()) != *action.cmx() {
        return Err(Error::Mismatch("the note is not the one this action commits to (the note commitments differ)".into()));
    }
    // the pools differ in the note plaintext's lead byte: each is decrypted in its own domain
    macro_rules! recover {
        ($domain:ty) => {{
            let domain = <$domain>::for_action(action);
            let esk = <$domain>::derive_esk(&note).ok_or_else(|| Error::Mismatch("no ephemeral key for this note".into()))?;
            try_output_recovery_with_pkd_esk(&domain, <$domain>::get_pk_d(&note), esk, action)
        }};
    }
    let recovered = match proof.pool {
        Pool::Orchard => recover!(OrchardDomain),
        Pool::Ironwood => recover!(IronwoodDomain),
    };
    let (got, to, memo) = recovered.ok_or_else(|| Error::Mismatch("the action's ciphertext does not decrypt to this note".into()))?;
    if to.to_raw_address_bytes() != proof.receiver || got.value().inner() != proof.value || got.rseed().as_bytes() != &proof.rseed {
        return Err(Error::Mismatch("the action's ciphertext decrypts to another note".into()));
    }
    Ok(Delivery { txid: proof.txid, wtxid: wtxid(&t), pool: proof.pool, action: proof.action, receiver: proof.receiver, value: proof.value, memo })
}

/// A checked delivery in a form that serialises to JSON: hex strings, the receiver as a unified address.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryView {
    /// displayed (byte-reversed) hex
    pub txid: String,
    pub wtxid: String,
    pub pool: Pool,
    pub action: u16,
    /// the receiver alone, as a unified address
    pub address: String,
    pub value: u64,
    pub memo_hex: String,
    pub memo_text: Option<String>,
}

impl Delivery {
    /// The txid as explorers display it.
    pub fn txid_hex(&self) -> String {
        let mut t = self.txid;
        t.reverse();
        hex::encode(t)
    }

    /// The delivery as JSON-friendly values, with the receiver in `network`'s address encoding.
    pub fn view(&self, network: NetworkType) -> DeliveryView {
        DeliveryView {
            txid: self.txid_hex(),
            wtxid: hex::encode(self.wtxid),
            pool: self.pool,
            action: self.action,
            address: crate::memo::receiver_address(&self.receiver, network),
            value: self.value,
            memo_hex: hex::encode(self.memo),
            memo_text: crate::memo::memo_text(&self.memo),
        }
    }
}
