//! Viewing keys: a unified full viewing key (UFVK) proves notes received and notes sent; a unified
//! incoming viewing key (UIVK) proves notes received only.
use orchard::keys::{FullViewingKey, IncomingViewingKey, OutgoingViewingKey, Scope};
use zcash_address::unified::{self, Container, Encoding};
use zcash_protocol::consensus::NetworkType;

use crate::Error;

/// The Orchard keys in a unified viewing key. Orchard keys also view the Ironwood pool.
#[derive(Clone, Debug)]
pub struct ViewingKeys {
    pub network: NetworkType,
    /// incoming viewing keys, external then internal scope (a UIVK has only the external one)
    pub incoming: Vec<IncomingViewingKey>,
    /// outgoing viewing keys, external then internal scope (empty for a UIVK)
    pub outgoing: Vec<OutgoingViewingKey>,
}

impl ViewingKeys {
    /// Reads a UFVK (`uview…`) or UIVK (`uivk…`); the key's Orchard item is required.
    pub fn parse(s: &str) -> Result<ViewingKeys, Error> {
        let s = s.trim();
        if let Ok((network, ufvk)) = unified::Ufvk::decode(s) {
            let fvk = ufvk
                .items()
                .into_iter()
                .find_map(|i| if let unified::Fvk::Orchard(b) = i { FullViewingKey::from_bytes(&b) } else { None })
                .ok_or_else(|| Error::Key("this unified full viewing key has no Orchard item".into()))?;
            return Ok(ViewingKeys::from_full_viewing_key(&fvk, network));
        }
        if let Ok((network, uivk)) = unified::Uivk::decode(s) {
            let ivk = uivk
                .items()
                .into_iter()
                .find_map(|i| if let unified::Ivk::Orchard(b) = i { Option::from(IncomingViewingKey::from_bytes(&b)) } else { None })
                .ok_or_else(|| Error::Key("this unified incoming viewing key has no Orchard item".into()))?;
            return Ok(ViewingKeys { network, incoming: vec![ivk], outgoing: vec![] });
        }
        Err(Error::Key("not a unified full viewing key (uview…) or incoming viewing key (uivk…)".into()))
    }

    /// Both scopes of an Orchard full viewing key.
    pub fn from_full_viewing_key(fvk: &FullViewingKey, network: NetworkType) -> ViewingKeys {
        ViewingKeys {
            network,
            incoming: vec![fvk.to_ivk(Scope::External), fvk.to_ivk(Scope::Internal)],
            outgoing: vec![fvk.to_ovk(Scope::External), fvk.to_ovk(Scope::Internal)],
        }
    }
}
