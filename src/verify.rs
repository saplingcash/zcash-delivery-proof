//! The `verify-bundle` feature: checks the bundle's zk-SNARK and signatures offline, as a node does,
//! with orchard's `BatchValidator`. It builds the verifying key for the bundle's circuit version once per
//! process (seconds, and tens of MB), so it is off by default: a mined transaction's bundles are valid.
//!
//! The signatures sign the transaction's shielded signature hash (ZIP 244), which for a transaction with
//! transparent inputs commits to the values and scripts of the coins they spend. Those are not in the
//! transaction, so the caller gives them, as a node has them.
use std::sync::OnceLock;

use orchard::bundle::BatchValidator;
use orchard::circuit::{OrchardCircuitVersion, VerifyingKey};
use rand_core::OsRng;
use zcash_primitives::transaction::sighash::{signature_hash, SignableInput};
use zcash_primitives::transaction::txid::TxIdDigester;
use zcash_primitives::transaction::{Authorization, TransactionData};
use zcash_protocol::value::Zatoshis;
use zcash_transparent::address::Script;
use zcash_transparent::sighash::TransparentAuthorizingContext;

use crate::tx::{bundle, read_tx};
use crate::{Error, Pool};

fn verifying_key(version: OrchardCircuitVersion) -> &'static VerifyingKey {
    static INSECURE_PRE_NU6_2: OnceLock<VerifyingKey> = OnceLock::new();
    static FIXED_POST_NU6_2: OnceLock<VerifyingKey> = OnceLock::new();
    static POST_NU6_3: OnceLock<VerifyingKey> = OnceLock::new();
    let cell = match version {
        OrchardCircuitVersion::InsecurePreNu6_2 => &INSECURE_PRE_NU6_2,
        OrchardCircuitVersion::FixedPostNu6_2 => &FIXED_POST_NU6_2,
        OrchardCircuitVersion::PostNu6_3 => &POST_NU6_3,
    };
    cell.get_or_init(|| VerifyingKey::build(version))
}

/// A coin a transparent input spends: its value and its scriptPubKey.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpentCoin {
    pub value: u64,
    pub script_pubkey: Vec<u8>,
}

#[derive(Debug)]
struct Spent {
    amounts: Vec<Zatoshis>,
    scripts: Vec<Script>,
}
impl zcash_transparent::bundle::Authorization for Spent {
    type ScriptSig = Script;
}
impl TransparentAuthorizingContext for Spent {
    fn input_amounts(&self) -> Vec<Zatoshis> {
        self.amounts.clone()
    }
    fn input_scriptpubkeys(&self) -> Vec<Script> {
        self.scripts.clone()
    }
}
struct ShieldedSighash;
impl Authorization for ShieldedSighash {
    type TransparentAuth = Spent;
    type SaplingAuth = sapling::bundle::Authorized;
    type OrchardAuth = orchard::bundle::Authorized;
}

/// Verifies the zk-SNARK, every spend-authorisation signature and the binding signature of the
/// transaction's bundle in `pool`, against the transaction's shielded signature hash. `spent`: the coins
/// its transparent inputs spend, in input order (empty for a transaction without transparent inputs). A
/// wrong value or script can only make a valid bundle fail, never an invalid one pass.
pub fn verify_bundle(tx: &[u8], pool: Pool, spent: &[SpentCoin]) -> Result<(), Error> {
    let t = read_tx(tx)?;
    let b = bundle(&t, pool).ok_or_else(|| Error::Mismatch(format!("the transaction has no {pool:?} bundle")))?;
    let inputs = t.transparent_bundle().map(|tb| tb.vin.len()).unwrap_or(0);
    if spent.len() != inputs {
        return Err(Error::Mismatch(format!("{} spent coins given for {inputs} transparent inputs", spent.len())));
    }
    let amounts = spent.iter().map(|c| Zatoshis::from_u64(c.value).map_err(|_| Error::Mismatch("a spent coin's value is out of range".into()))).collect::<Result<Vec<_>, _>>()?;
    let scripts: Vec<Script> = spent.iter().map(|c| Script(zcash_script::script::Code(c.script_pubkey.clone()))).collect();
    let data: TransactionData<ShieldedSighash> = t.clone().into_data().map_bundles(
        |tb| {
            tb.map(|tb| zcash_transparent::bundle::Bundle {
                vin: tb.vin.iter().map(|i| zcash_transparent::bundle::TxIn::from_parts(i.prevout().clone(), i.script_sig().clone(), i.sequence())).collect(),
                vout: tb.vout.clone(),
                authorization: Spent { amounts: amounts.clone(), scripts: scripts.clone() },
            })
        },
        |s| s,
        |o| o,
    );
    let parts = data.digest(TxIdDigester);
    let sighash: [u8; 32] = *signature_hash(&data, &SignableInput::Shielded, &parts).as_ref();
    let mut v = BatchValidator::new(verifying_key(b.bundle_version().circuit_version()));
    v.add_bundle(b, sighash).map_err(|e| Error::Mismatch(format!("the bundle cannot be verified with its circuit's key: {e:?}")))?;
    if v.validate(OsRng) {
        Ok(())
    } else {
        Err(Error::Mismatch(format!("the {pool:?} bundle's zk-SNARK or signatures do not verify")))
    }
}
