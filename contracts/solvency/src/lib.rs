#![no_std]
//! Solvent — ZK proof-of-reserves verifier for Stellar issuers.
//!
//! An issuer submits a Groth16 proof generated off-chain from the
//! `solvency.circom` circuit: the private customer balances sum to a public
//! `total`, and every balance is a non-negative 64-bit amount. This contract
//! verifies the proof on-chain and records whether the attested reserve covers
//! the proven liabilities.
//!
//! Two of Stellar's native ZK primitives are load-bearing here:
//!   * `attest`        verifies with **BLS12-381** pairing (Protocol 22 / CAP-0059)
//!   * `attest_bn254`  verifies with **BN254** pairing (Protocol 26 / CAP-0074)
//! Either path produces the same solvency attestation. Verification keys are
//! embedded at build time (`vk_data.rs`, `vk_data_bn254.rs`).
//!
//! Attestations are stored per issuer, so one issuer can never overwrite or
//! grief another's published verdict.

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype,
    crypto::bls12_381::{Fr, G1Affine, G2Affine},
    crypto::bn254::{Bn254G1Affine, Bn254G2Affine, Fr as BnFr},
    symbol_short, vec, Address, Bytes, BytesN, Env, Symbol, U256, Vec,
};

mod vk_data;
mod vk_data_bn254;
#[cfg(test)]
mod test;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    MalformedVerifyingKey = 1,
    ProofRejected = 2,
}

#[contracttype]
#[derive(Clone)]
pub struct Attestation {
    pub issuer: Address,
    pub total: u128,   // proven sum of customer liabilities
    pub reserve: u128, // issuer's attested reserves
    pub solvent: bool, // proof valid AND reserve >= total
    pub curve: Symbol, // which primitive verified it: BLS12_381 or BN254
    pub ledger: u32,
}

#[contracttype]
pub enum DataKey {
    Attest(Address), // per-issuer attestation
    Count,           // global attestation counter
}

fn u256_from_u128(env: &Env, v: u128) -> U256 {
    let mut be = [0u8; 32];
    be[16..].copy_from_slice(&v.to_be_bytes());
    U256::from_be_bytes(env, &Bytes::from_array(env, &be))
}

fn load_vk_bls(env: &Env) -> (G1Affine, G2Affine, G2Affine, G2Affine, Vec<G1Affine>) {
    let ic = vec![
        env,
        G1Affine::from_array(env, &vk_data::VK_IC0),
        G1Affine::from_array(env, &vk_data::VK_IC1),
    ];
    (
        G1Affine::from_array(env, &vk_data::VK_ALPHA),
        G2Affine::from_array(env, &vk_data::VK_BETA),
        G2Affine::from_array(env, &vk_data::VK_GAMMA),
        G2Affine::from_array(env, &vk_data::VK_DELTA),
        ic,
    )
}

fn record(env: &Env, issuer: Address, total: u128, reserve: u128, curve: Symbol) -> bool {
    let solvent = reserve >= total;
    let att = Attestation {
        issuer: issuer.clone(),
        total,
        reserve,
        solvent,
        curve: curve.clone(),
        ledger: env.ledger().sequence(),
    };
    env.storage().persistent().set(&DataKey::Attest(issuer.clone()), &att);
    let count: u32 = env.storage().instance().get(&DataKey::Count).unwrap_or(0);
    env.storage().instance().set(&DataKey::Count, &(count + 1));
    env.events()
        .publish((symbol_short!("attest"), issuer), (total, reserve, solvent, curve));
    solvent
}

#[contract]
pub struct SolvencyContract;

#[contractimpl]
impl SolvencyContract {
    /// Verify a solvency proof with **BLS12-381** and record the attestation.
    pub fn attest(
        env: Env,
        issuer: Address,
        total: u128,
        reserve: u128,
        proof_a: BytesN<96>,
        proof_b: BytesN<192>,
        proof_c: BytesN<96>,
    ) -> Result<bool, Error> {
        issuer.require_auth();
        let (alpha, beta, gamma, delta, ic) = load_vk_bls(&env);
        let pub_signals = vec![&env, Fr::from_u256(u256_from_u128(&env, total))];
        if pub_signals.len() + 1 != ic.len() {
            return Err(Error::MalformedVerifyingKey);
        }
        let bls = env.crypto().bls12_381();
        let a = G1Affine::from_bytes(proof_a);
        let b = G2Affine::from_bytes(proof_b);
        let c = G1Affine::from_bytes(proof_c);
        let mut vk_x = ic.get(0).unwrap();
        for (s, v) in pub_signals.iter().zip(ic.iter().skip(1)) {
            vk_x = bls.g1_add(&vk_x, &bls.g1_mul(&v, &s));
        }
        let vp1 = vec![&env, -a, alpha, vk_x, c];
        let vp2 = vec![&env, b, beta, gamma, delta];
        if !bls.pairing_check(vp1, vp2) {
            return Err(Error::ProofRejected);
        }
        Ok(record(&env, issuer, total, reserve, symbol_short!("BLS12_381")))
    }

    /// Verify a solvency proof with **BN254** (Protocol 26 host functions) and
    /// record the attestation. Same guarantee, second native primitive.
    pub fn attest_bn254(
        env: Env,
        issuer: Address,
        total: u128,
        reserve: u128,
        proof_a: BytesN<64>,
        proof_b: BytesN<128>,
        proof_c: BytesN<64>,
    ) -> Result<bool, Error> {
        issuer.require_auth();
        let ic = vec![
            &env,
            Bn254G1Affine::from_bytes(BytesN::from_array(&env, &vk_data_bn254::VK_IC0)),
            Bn254G1Affine::from_bytes(BytesN::from_array(&env, &vk_data_bn254::VK_IC1)),
        ];
        let alpha = Bn254G1Affine::from_bytes(BytesN::from_array(&env, &vk_data_bn254::VK_ALPHA));
        let beta = Bn254G2Affine::from_bytes(BytesN::from_array(&env, &vk_data_bn254::VK_BETA));
        let gamma = Bn254G2Affine::from_bytes(BytesN::from_array(&env, &vk_data_bn254::VK_GAMMA));
        let delta = Bn254G2Affine::from_bytes(BytesN::from_array(&env, &vk_data_bn254::VK_DELTA));

        let pub_signals = vec![&env, BnFr::from_u256(u256_from_u128(&env, total))];
        if pub_signals.len() + 1 != ic.len() {
            return Err(Error::MalformedVerifyingKey);
        }
        let bn = env.crypto().bn254();
        let a = Bn254G1Affine::from_bytes(proof_a);
        let b = Bn254G2Affine::from_bytes(proof_b);
        let c = Bn254G1Affine::from_bytes(proof_c);
        let mut vk_x = ic.get(0).unwrap();
        for (s, v) in pub_signals.iter().zip(ic.iter().skip(1)) {
            vk_x = bn.g1_add(&vk_x, &bn.g1_mul(&v, &s));
        }
        let vp1 = vec![&env, -a, alpha, vk_x, c];
        let vp2 = vec![&env, b, beta, gamma, delta];
        if !bn.pairing_check(vp1, vp2) {
            return Err(Error::ProofRejected);
        }
        Ok(record(&env, issuer, total, reserve, symbol_short!("BN254")))
    }

    /// Latest recorded attestation for a specific issuer.
    pub fn latest(env: Env, issuer: Address) -> Option<Attestation> {
        env.storage().persistent().get(&DataKey::Attest(issuer))
    }

    /// True if this issuer's most recent attestation proved solvency.
    pub fn is_solvent(env: Env, issuer: Address) -> bool {
        let att: Option<Attestation> = env.storage().persistent().get(&DataKey::Attest(issuer));
        att.map(|a| a.solvent).unwrap_or(false)
    }

    /// Total number of attestations recorded across all issuers.
    pub fn count(env: Env) -> u32 {
        env.storage().instance().get(&DataKey::Count).unwrap_or(0)
    }
}
