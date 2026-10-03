//! Delivery proofs for Zcash shielded payments.
//!
//! A delivery proof shows that a transaction delivers one note, in the Orchard, Ironwood or Sapling
//! pool, to one receiver, with a value and a memo, and nothing else about the wallet: no viewing key, no
//! other note. The receiving wallet makes it with its incoming viewing key, or the sending wallet with its
//! outgoing viewing key; anyone checks it against the transaction's bytes with no key at all.
//!
//! ```no_run
//! use zcash_delivery_proof::{check, make, memo_text, DeliveryProof, ViewingKeys};
//! # fn run(tx: &[u8], ufvk: &str, text: &str) -> Result<(), zcash_delivery_proof::Error> {
//! // the wallet: every note this key received or sent in the transaction, each with its proof
//! let keys = ViewingKeys::parse(ufvk)?;
//! for found in make(tx, &keys)? {
//!     println!("{:?} {}", found.side, found.proof.encode());
//! }
//! // anyone: check a proof against the transaction's bytes
//! let delivery = check(tx, &DeliveryProof::decode(text)?)?;
//! println!("{} zatoshi, memo {:?}", delivery.value, memo_text(&delivery.memo));
//! # Ok(())
//! # }
//! ```
//!
//! What a check does not show: that the transaction is mined (compare its txid or wtxid with a node),
//! who sent it, and, unless the `verify-bundle` feature is used (Orchard and Ironwood bundles), that its
//! zk-SNARK and signatures hold. See SPEC.md.

mod check;
mod error;
mod keys;
mod make;
mod memo;
mod proof;
mod tx;
#[cfg(feature = "verify-bundle")]
mod verify;

pub use check::{check, Delivery, DeliveryView};
pub use error::Error;
pub use keys::ViewingKeys;
pub use make::{make, Found, Side};
pub use memo::{address_has_pool_receiver, address_has_receiver, memo_text, pool_receiver_address, receiver_address};
pub use proof::{DeliveryProof, Pool, PREFIX, PROOF_LEN};
pub use tx::{header_branch, read_tx, wtxid};
#[cfg(feature = "verify-bundle")]
pub use verify::{verify_bundle, SpentCoin};
