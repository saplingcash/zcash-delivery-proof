//! Test keys, values and transactions. Every key and value is derived from a label under
//! "sapling.cash/zdp/": the first bytes of SHA-512("sapling.cash/zdp/" + name). The transactions are built
//! by librustzcash's own builder with an RNG seeded from a label, so they are the same on every run.
#![allow(dead_code)]
use orchard::keys::{FullViewingKey, Scope, SpendingKey};
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha20Rng;
use sha2::{Digest, Sha512};
use zcash_address::unified::{self, Encoding};
use zcash_primitives::transaction::builder::{BuildConfig, Builder, BundlePadding};
use zcash_primitives::transaction::fees::zip317::FeeRule;
use zcash_protocol::consensus::{BlockHeight, NetworkType, NetworkUpgrade, Parameters, TestNetwork};
use zcash_protocol::memo::MemoBytes;
use zcash_protocol::value::Zatoshis;
use zcash_transparent::address::TransparentAddress;
use zcash_transparent::builder::TransparentSigningSet;
use zcash_transparent::bundle::{OutPoint, TxOut};

pub const PREFIX: &str = "sapling.cash/zdp/";

/// The first `N` bytes (at most 64) of SHA-512(PREFIX + name).
pub fn label<const N: usize>(name: &str) -> [u8; N] {
    Sha512::digest(format!("{PREFIX}{name}").as_bytes())[..N].try_into().expect("at most 64 bytes")
}

/// An Orchard spending key: the label's bytes, or those of "name#1", "name#2", … for the rare value that
/// is not a valid key.
pub fn spending_key(name: &str) -> SpendingKey {
    (0u32..)
        .find_map(|i| Option::from(SpendingKey::from_bytes(label::<32>(&if i == 0 { name.to_string() } else { format!("{name}#{i}") }))))
        .expect("a valid spending key")
}

pub fn fvk(name: &str) -> FullViewingKey {
    FullViewingKey::from(&spending_key(name))
}

pub fn address(name: &str) -> orchard::Address {
    fvk(name).address_at(0u32, Scope::External)
}

pub fn ufvk(name: &str) -> String {
    unified::Ufvk::try_from_items(vec![unified::Fvk::Orchard(fvk(name).to_bytes())]).expect("a UFVK").encode(&NetworkType::Test)
}

pub fn uivk(name: &str) -> String {
    unified::Uivk::try_from_items(vec![unified::Ivk::Orchard(fvk(name).to_ivk(Scope::External).to_bytes())]).expect("a UIVK").encode(&NetworkType::Test)
}

/// The builder takes Sapling provers; these transactions have no Sapling part, so they are never called.
pub struct NoSapling;
impl sapling::prover::SpendProver for NoSapling {
    type Proof = sapling::bundle::GrothProofBytes;
    #[allow(clippy::too_many_arguments)]
    fn prepare_circuit(_: sapling::ProofGenerationKey, _: sapling::Diversifier, _: sapling::Rseed, _: sapling::value::NoteValue, _: jubjub::Fr, _: sapling::value::ValueCommitTrapdoor, _: bls12_381::Scalar, _: sapling::MerklePath) -> Option<sapling::circuit::Spend> {
        None
    }
    fn create_proof<R: rand_core::RngCore>(&self, _: sapling::circuit::Spend, _: &mut R) -> Self::Proof {
        unreachable!("no Sapling spends")
    }
    fn encode_proof(p: Self::Proof) -> sapling::bundle::GrothProofBytes {
        p
    }
}
impl sapling::prover::OutputProver for NoSapling {
    type Proof = sapling::bundle::GrothProofBytes;
    fn prepare_circuit(_: &sapling::keys::EphemeralSecretKey, _: sapling::PaymentAddress, _: jubjub::Fr, _: sapling::value::NoteValue, _: sapling::value::ValueCommitTrapdoor) -> sapling::circuit::Output {
        unreachable!("no Sapling outputs")
    }
    fn create_proof<R: rand_core::RngCore>(&self, _: sapling::circuit::Output, _: &mut R) -> Self::Proof {
        unreachable!("no Sapling outputs")
    }
    fn encode_proof(p: Self::Proof) -> sapling::bundle::GrothProofBytes {
        p
    }
}

/// The scriptPubKey of the coin a test transaction spends (the P2PKH of its label-derived key).
pub fn coin_script(name: &str) -> Vec<u8> {
    let secret = secp256k1::SecretKey::from_slice(&label::<32>(&format!("{name}/coin-key"))).expect("a secp256k1 key");
    let pk = secp256k1::PublicKey::from_secret_key(&secp256k1::Secp256k1::new(), &secret);
    let script: zcash_transparent::address::Script = TransparentAddress::from_pubkey(&pk).script().into();
    script.0 .0
}

/// The value of the coin every test transaction spends.
pub const COIN_VALUE: u64 = 1_000_000;

/// One payment in a test transaction.
pub struct Payment {
    /// the recipient's key label
    pub to: &'static str,
    pub value: u64,
    pub memo: &'static str,
}

/// A test transaction under Zcash testnet rules, from a transparent coin (a label-derived key) to one or
/// more shielded payments in `pool`, each with the sender's outgoing viewing key, and transparent change:
/// - Orchard: a v5 transaction at the first NU6.2 height (Orchard payments to others were made before
///   NU6.3; NU6.3 restricts new ones);
/// - Ironwood: a v6 transaction at the first NU6.3 height.
pub fn build(pool: zcash_delivery_proof::Pool, name: &str, sender: &str, payments: &[Payment]) -> Vec<u8> {
    let params = TestNetwork;
    let height: BlockHeight = match pool {
        zcash_delivery_proof::Pool::Orchard => params.activation_height(NetworkUpgrade::Nu6_2).unwrap(),
        zcash_delivery_proof::Pool::Ironwood => params.activation_height(NetworkUpgrade::Nu6_3).unwrap(),
    };
    let mut signing = TransparentSigningSet::new();
    let secret = secp256k1::SecretKey::from_slice(&label::<32>(&format!("{name}/coin-key"))).expect("a secp256k1 key");
    let pubkey = signing.add_key(secret);
    let coin_addr = TransparentAddress::from_pubkey(&pubkey);
    let coin_value = COIN_VALUE;
    let coin = TxOut::new(Zatoshis::from_u64(coin_value).unwrap(), coin_addr.script().into());
    let outpoint = OutPoint::new(label::<32>(&format!("{name}/coin")), 0);
    let ovk = fvk(sender).to_ovk(Scope::External);
    let rule = FeeRule::standard();
    let anchor = orchard::Anchor::empty_tree();
    let assemble = |change: u64| {
        let config = match pool {
            zcash_delivery_proof::Pool::Orchard => BuildConfig::Standard { sapling_anchor: None, orchard_anchor: Some(anchor), ironwood_anchor: None, orchard_padding: BundlePadding::DEFAULT, ironwood_padding: BundlePadding::DEFAULT },
            zcash_delivery_proof::Pool::Ironwood => BuildConfig::Standard { sapling_anchor: None, orchard_anchor: None, ironwood_anchor: Some(anchor), orchard_padding: BundlePadding::DEFAULT, ironwood_padding: BundlePadding::DEFAULT },
        };
        let mut b: Builder<TestNetwork, ()> = Builder::new(params, height, config).with_expiry_height(height + 40);
        b.add_transparent_p2pkh_input(pubkey, outpoint.clone(), coin.clone()).unwrap();
        for p in payments {
            let memo = MemoBytes::from_bytes(p.memo.as_bytes()).unwrap();
            let v = Zatoshis::from_u64(p.value).unwrap();
            match pool {
                zcash_delivery_proof::Pool::Orchard => b.add_orchard_output::<std::convert::Infallible>(Some(ovk.clone()), address(p.to), v, memo).unwrap(),
                zcash_delivery_proof::Pool::Ironwood => b.add_ironwood_output::<std::convert::Infallible>(Some(ovk.clone()), address(p.to), v, memo).unwrap(),
            }
        }
        b.add_transparent_output(&coin_addr, Zatoshis::from_u64(change).unwrap()).unwrap();
        b
    };
    let fee = u64::from(assemble(10_000).get_fee(&rule).unwrap());
    let paid: u64 = payments.iter().map(|p| p.value).sum();
    let rng = ChaCha20Rng::from_seed(label::<32>(&format!("{name}/rng")));
    let result = assemble(coin_value - paid - fee).build(&signing, &[], &[], rng, &NoSapling, &NoSapling, &rule).expect("the test transaction builds");
    let mut bytes = Vec::new();
    result.transaction().write(&mut bytes).unwrap();
    bytes
}
