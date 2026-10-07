//! Memos (ZIP 302) and receivers as addresses (ZIP 316).
use zcash_address::unified::{self, Container, Encoding};
use zcash_address::{ConversionError, TryFromAddress, ZcashAddress};
use zcash_protocol::consensus::NetworkType;
use zcash_protocol::memo::{Memo, MemoBytes};

use crate::Pool;

/// The memo as text, if it is a text memo (ZIP 302: a first byte up to 0xF4, valid UTF-8, zero-padded).
pub fn memo_text(memo: &[u8; 512]) -> Option<String> {
    match Memo::try_from(MemoBytes::from_bytes(memo).ok()?).ok()? {
        Memo::Text(t) => Some(String::from(t)),
        _ => None,
    }
}

/// A raw Orchard-family receiver as a unified address that holds only it (ZIP 316).
pub fn receiver_address(receiver: &[u8; 43], network: NetworkType) -> String {
    pool_receiver_address(Pool::Orchard, receiver, network)
}

/// A proof's raw receiver as a unified address that holds only it (ZIP 316): an Orchard receiver for
/// the Orchard and Ironwood pools, a Sapling receiver for the Sapling pool.
pub fn pool_receiver_address(pool: Pool, receiver: &[u8; 43], network: NetworkType) -> String {
    let item = match pool {
        Pool::Orchard | Pool::Ironwood => unified::Receiver::Orchard(*receiver),
        Pool::Sapling => unified::Receiver::Sapling(*receiver),
    };
    unified::Address::try_from_items(unified::Revision::R0, vec![unified::Uitem::Data(item)]).expect("one shielded receiver is a valid unified address").encode(&network)
}

/// Whether a unified address (or any address zcash_address reads) carries this Orchard receiver. A
/// proof names the receiver alone; the full address the payer was given may carry other receivers too.
pub fn address_has_receiver(address: &str, receiver: &[u8; 43]) -> bool {
    address_has_pool_receiver(address, Pool::Orchard, receiver)
}

/// Whether an address carries this receiver of `pool`: a unified address with that receiver among its
/// items, or, for Sapling, a Sapling address (`zs…`) that is exactly it.
pub fn address_has_pool_receiver(address: &str, pool: Pool, receiver: &[u8; 43]) -> bool {
    struct Has(Vec<unified::Receiver>);
    impl TryFromAddress for Has {
        type Error = ();
        fn try_from_sapling(_: NetworkType, data: [u8; 43]) -> Result<Self, ConversionError<()>> {
            Ok(Has(vec![unified::Receiver::Sapling(data)]))
        }
        fn try_from_unified(_: NetworkType, data: unified::Address) -> Result<Self, ConversionError<()>> {
            Ok(Has(data.items()))
        }
    }
    let Ok(Has(items)) = ZcashAddress::try_from_encoded(address).map_err(|_| ()).and_then(|a| a.convert::<Has>().map_err(|_| ())) else {
        return false;
    };
    items.iter().any(|r| match (pool, r) {
        (Pool::Orchard | Pool::Ironwood, unified::Receiver::Orchard(b)) | (Pool::Sapling, unified::Receiver::Sapling(b)) => b == receiver,
        _ => false,
    })
}
