//! Reading a transaction: exactly the bytes given, or nothing.
use orchard::bundle::Authorized;
use zcash_primitives::transaction::Transaction;
use zcash_protocol::consensus::BranchId;
use zcash_protocol::value::ZatBalance;

use crate::{Error, Pool};

/// The consensus branch a transaction is read under. v5 and v6 transactions name theirs in the header;
/// a v4 transaction names none and is read under Canopy (its layout and txid are the same for every v4
/// branch). v4 transactions carry no Orchard or Ironwood bundle, but can still be read.
pub fn header_branch(tx: &[u8]) -> Result<BranchId, Error> {
    let short = || Error::Transaction("too short to be a transaction".into());
    let header = u32::from_le_bytes(tx.get(0..4).ok_or_else(short)?.try_into().unwrap());
    if header & 0x7fff_ffff == 4 {
        return Ok(BranchId::Canopy);
    }
    let id = u32::from_le_bytes(tx.get(8..12).ok_or_else(short)?.try_into().unwrap());
    BranchId::try_from(id).map_err(|_| Error::Transaction(format!("unknown consensus branch {id:#010x}")))
}

/// Parses a transaction with librustzcash and requires that it serialises back to exactly these bytes,
/// so the proof is checked against the bytes given and not a re-encoding of them.
pub fn read_tx(tx: &[u8]) -> Result<Transaction, Error> {
    let t = Transaction::read(tx, header_branch(tx)?).map_err(|e| Error::Transaction(format!("not a transaction: {e}")))?;
    let mut back = Vec::with_capacity(tx.len());
    t.write(&mut back).map_err(|e| Error::Transaction(e.to_string()))?;
    if back != tx {
        return Err(Error::Transaction("librustzcash reads these bytes but writes different ones: not a canonical transaction".into()));
    }
    Ok(t)
}

/// The transaction's bundle in `pool`, if it has one.
pub(crate) fn bundle(t: &Transaction, pool: Pool) -> Option<&orchard::Bundle<Authorized, ZatBalance>> {
    match pool {
        Pool::Orchard => t.orchard_bundle(),
        Pool::Ironwood => t.ironwood_bundle(),
    }
}

/// The wtxid (ZIP 239): the txid then the authorizing-data digest, both in internal byte order. Unlike
/// the txid it covers the signatures and proofs, so a node that shows it vouches for these exact bytes.
pub fn wtxid(t: &Transaction) -> [u8; 64] {
    let mut w = [0u8; 64];
    w[..32].copy_from_slice(t.txid().as_ref());
    w[32..].copy_from_slice(t.auth_commitment().as_bytes());
    w
}
