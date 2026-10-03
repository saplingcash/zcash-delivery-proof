//! The delivery proof and its encodings (SPEC.md §2).
//!
//! ```text
//! zdp:1:<base64url( txid 32 (internal order) || pool u8 (1 = Orchard, 2 = Ironwood, 3 = Sapling)
//!                   || action u16 LE || receiver 43 || value u64 LE || rseed 32 )>        (118 bytes)
//! ```
use base64::Engine;
use serde::Serialize;

use crate::Error;

/// The text prefix of an encoded proof: the format name and its version.
pub const PREFIX: &str = "zdp:1:";
/// The length of a proof's bytes.
pub const PROOF_LEN: usize = 32 + 1 + 2 + 43 + 8 + 32;

/// The shielded pool a note is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Pool {
    Orchard,
    Ironwood,
    /// The Sapling pool. A proof's "action" is then the output's index in the Sapling bundle.
    Sapling,
}

impl Pool {
    /// The pool's byte in a proof.
    pub fn to_byte(self) -> u8 {
        match self {
            Pool::Orchard => 1,
            Pool::Ironwood => 2,
            Pool::Sapling => 3,
        }
    }

    pub fn from_byte(b: u8) -> Option<Pool> {
        match b {
            1 => Some(Pool::Orchard),
            2 => Some(Pool::Ironwood),
            3 => Some(Pool::Sapling),
            _ => None,
        }
    }

    /// Whether the pool is Orchard or Ironwood (the Orchard protocol's actions and keys).
    pub fn is_orchard_family(self) -> bool {
        matches!(self, Pool::Orchard | Pool::Ironwood)
    }
}

/// One note, pinned to one action of one transaction: everything needed to rebuild it and check that
/// the action delivers exactly it. It reveals the receiver, the value and the memo, and nothing else
/// about the wallet: no key, no other note.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeliveryProof {
    /// the transaction's id, in internal byte order (the order it is hashed in, reversed from how
    /// explorers display it)
    pub txid: [u8; 32],
    pub pool: Pool,
    /// the action's index in the pool's bundle (for Sapling, the output's index)
    pub action: u16,
    /// the raw receiver, in the pool's own encoding (11-byte diversifier, 32-byte pk_d): an
    /// Orchard-family address, or a Sapling payment address
    pub receiver: [u8; 43],
    /// the note's value, in zatoshi
    pub value: u64,
    /// the note's rseed: it gives the note (with the action's rho in the Orchard family), and the esk its
    /// ciphertext was made with
    pub rseed: [u8; 32],
}

impl DeliveryProof {
    pub fn to_bytes(&self) -> [u8; PROOF_LEN] {
        let mut b = [0u8; PROOF_LEN];
        b[0..32].copy_from_slice(&self.txid);
        b[32] = self.pool.to_byte();
        b[33..35].copy_from_slice(&self.action.to_le_bytes());
        b[35..78].copy_from_slice(&self.receiver);
        b[78..86].copy_from_slice(&self.value.to_le_bytes());
        b[86..118].copy_from_slice(&self.rseed);
        b
    }

    pub fn from_bytes(b: &[u8]) -> Result<DeliveryProof, Error> {
        if b.len() != PROOF_LEN {
            return Err(Error::Proof(format!("a delivery proof is {PROOF_LEN} bytes, not {}", b.len())));
        }
        Ok(DeliveryProof {
            txid: b[0..32].try_into().unwrap(),
            pool: Pool::from_byte(b[32]).ok_or_else(|| Error::Proof(format!("pool {} is not Orchard (1), Ironwood (2) or Sapling (3)", b[32])))?,
            action: u16::from_le_bytes(b[33..35].try_into().unwrap()),
            receiver: b[35..78].try_into().unwrap(),
            value: u64::from_le_bytes(b[78..86].try_into().unwrap()),
            rseed: b[86..118].try_into().unwrap(),
        })
    }

    /// The proof as text: `zdp:1:` and its bytes in base64url without padding.
    pub fn encode(&self) -> String {
        format!("{PREFIX}{}", base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(self.to_bytes()))
    }

    pub fn decode(s: &str) -> Result<DeliveryProof, Error> {
        let body = s.trim().strip_prefix(PREFIX).ok_or_else(|| Error::Proof(format!("not a delivery proof: it must start with {PREFIX}")))?;
        let b = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(body).map_err(|_| Error::Proof("the proof's body is not base64url (no padding)".into()))?;
        DeliveryProof::from_bytes(&b)
    }

    /// The txid as explorers display it (byte-reversed hex).
    pub fn txid_hex(&self) -> String {
        let mut t = self.txid;
        t.reverse();
        hex::encode(t)
    }
}
