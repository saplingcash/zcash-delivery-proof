//! Memos (ZIP 302).
use zcash_address::unified::{self, Encoding};
use zcash_protocol::consensus::NetworkType;
use zcash_protocol::memo::{Memo, MemoBytes};

/// The memo as text, if it is a text memo (ZIP 302: a first byte up to 0xF4, valid UTF-8, zero-padded).
pub fn memo_text(memo: &[u8; 512]) -> Option<String> {
    match Memo::try_from(MemoBytes::from_bytes(memo).ok()?).ok()? {
        Memo::Text(t) => Some(String::from(t)),
        _ => None,
    }
}

/// A raw Orchard-family receiver as a unified address that holds only it (ZIP 316).
pub fn receiver_address(receiver: &[u8; 43], network: NetworkType) -> String {
    unified::Address::try_from_items(vec![unified::Receiver::Orchard(*receiver)]).expect("one Orchard receiver is a valid unified address").encode(&network)
}

/// Whether a unified address (or any address zcash_address reads) carries this Orchard receiver. A
/// proof names the receiver alone; the full address the payer was given may carry other receivers too.
pub fn address_has_receiver(address: &str, receiver: &[u8; 43]) -> bool {
    use zcash_address::unified::Container;
    match unified::Address::decode(address) {
        Ok((_, ua)) => ua.items().iter().any(|r| matches!(r, unified::Receiver::Orchard(b) if b == receiver)),
        Err(_) => false,
    }
}
