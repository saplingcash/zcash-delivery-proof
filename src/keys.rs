//! Viewing keys: a unified full viewing key (UFVK) proves notes received and notes sent; a unified
//! incoming viewing key (UIVK) proves notes received only. Its Orchard item sees the Orchard and the
//! Ironwood pools, its Sapling item the Sapling pool.
use orchard::keys::{FullViewingKey, IncomingViewingKey, OutgoingViewingKey, Scope};
use sapling::keys::{OutgoingViewingKey as SaplingOutgoingViewingKey, PreparedIncomingViewingKey as SaplingIncomingViewingKey};
use sapling::zip32::{DiversifiableFullViewingKey, IncomingViewingKey as SaplingUivkItem};
use zcash_address::unified::{self, Container, Encoding};
use zcash_protocol::consensus::NetworkType;

use crate::Error;

/// The Orchard and Sapling keys in a unified viewing key. Orchard keys also view the Ironwood pool.
#[derive(Clone, Debug)]
pub struct ViewingKeys {
    pub network: NetworkType,
    /// Orchard incoming viewing keys, external then internal scope (a UIVK has only the external one)
    pub incoming: Vec<IncomingViewingKey>,
    /// Orchard outgoing viewing keys, external then internal scope (empty for a UIVK)
    pub outgoing: Vec<OutgoingViewingKey>,
    /// Sapling incoming viewing keys, external then internal scope (a UIVK has only the external one)
    pub sapling_incoming: Vec<SaplingIncomingViewingKey>,
    /// Sapling outgoing viewing keys, external then internal scope (empty for a UIVK)
    pub sapling_outgoing: Vec<SaplingOutgoingViewingKey>,
}

impl ViewingKeys {
    /// Reads a UFVK (`uview…`) or UIVK (`uivk…`); an Orchard item, a Sapling item, or both are required.
    pub fn parse(s: &str) -> Result<ViewingKeys, Error> {
        let s = s.trim();
        if let Ok((network, ufvk)) = unified::Ufvk::decode(s) {
            let mut keys = ViewingKeys::none(network);
            for item in ufvk.items() {
                match item {
                    unified::Fvk::Orchard(b) => {
                        let fvk = FullViewingKey::from_bytes(&b).ok_or_else(|| Error::Key("this unified full viewing key's Orchard item is not a valid key".into()))?;
                        keys.add_orchard(&fvk);
                    }
                    unified::Fvk::Sapling(b) => {
                        let dfvk = DiversifiableFullViewingKey::from_bytes(&b).ok_or_else(|| Error::Key("this unified full viewing key's Sapling item is not a valid key".into()))?;
                        keys.add_sapling(&dfvk);
                    }
                    _ => {}
                }
            }
            if keys.incoming.is_empty() && keys.sapling_incoming.is_empty() {
                return Err(Error::Key("this unified full viewing key has no Orchard or Sapling item".into()));
            }
            return Ok(keys);
        }
        if let Ok((network, uivk)) = unified::Uivk::decode(s) {
            let mut keys = ViewingKeys::none(network);
            for item in uivk.items() {
                match item {
                    unified::Ivk::Orchard(b) => keys.incoming.push(Option::from(IncomingViewingKey::from_bytes(&b)).ok_or_else(|| Error::Key("this unified incoming viewing key's Orchard item is not a valid key".into()))?),
                    unified::Ivk::Sapling(b) => keys.sapling_incoming.push(Option::<SaplingUivkItem>::from(SaplingUivkItem::from_bytes(&b)).ok_or_else(|| Error::Key("this unified incoming viewing key's Sapling item is not a valid key".into()))?.prepare()),
                    _ => {}
                }
            }
            if keys.incoming.is_empty() && keys.sapling_incoming.is_empty() {
                return Err(Error::Key("this unified incoming viewing key has no Orchard or Sapling item".into()));
            }
            return Ok(keys);
        }
        Err(Error::Key("not a unified full viewing key (uview…) or incoming viewing key (uivk…)".into()))
    }

    /// No keys at all, for `network`: add some with the methods below.
    pub fn none(network: NetworkType) -> ViewingKeys {
        ViewingKeys { network, incoming: vec![], outgoing: vec![], sapling_incoming: vec![], sapling_outgoing: vec![] }
    }

    /// Both scopes of an Orchard full viewing key.
    pub fn from_full_viewing_key(fvk: &FullViewingKey, network: NetworkType) -> ViewingKeys {
        let mut keys = ViewingKeys::none(network);
        keys.add_orchard(fvk);
        keys
    }

    /// Both scopes of a Sapling (diversifiable) full viewing key.
    pub fn from_sapling_full_viewing_key(dfvk: &DiversifiableFullViewingKey, network: NetworkType) -> ViewingKeys {
        let mut keys = ViewingKeys::none(network);
        keys.add_sapling(dfvk);
        keys
    }

    /// Outgoing viewing keys alone, given as their 32 bytes: they find the notes a sender built with them
    /// (an Orchard OVK in the Orchard and Ironwood pools, a Sapling OVK in the Sapling pool), and nothing
    /// received. An outgoing viewing key is 32 bytes in both protocols; a sender may publish one so that
    /// anyone can find, and prove, the notes it sent with it.
    pub fn from_outgoing_keys(network: NetworkType, orchard: &[[u8; 32]], sapling: &[[u8; 32]]) -> ViewingKeys {
        let mut keys = ViewingKeys::none(network);
        keys.outgoing = orchard.iter().map(|b| OutgoingViewingKey::from(*b)).collect();
        keys.sapling_outgoing = sapling.iter().map(|b| SaplingOutgoingViewingKey(*b)).collect();
        keys
    }

    fn add_orchard(&mut self, fvk: &FullViewingKey) {
        self.incoming.extend([fvk.to_ivk(Scope::External), fvk.to_ivk(Scope::Internal)]);
        self.outgoing.extend([fvk.to_ovk(Scope::External), fvk.to_ovk(Scope::Internal)]);
    }

    fn add_sapling(&mut self, dfvk: &DiversifiableFullViewingKey) {
        for scope in [zip32::Scope::External, zip32::Scope::Internal] {
            self.sapling_incoming.push(SaplingIncomingViewingKey::new(&dfvk.to_ivk(scope)));
            self.sapling_outgoing.push(dfvk.to_ovk(scope));
        }
    }
}
