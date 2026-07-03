#![no_std]
//! Solvent — ZK proof-of-reserves verifier for Stellar issuers.
//!
//! An issuer submits a Groth16 (BLS12-381) proof generated off-chain from the
//! `solvency.circom` circuit. The proof attests that a set of private customer
//! balances sums to a public `total` and that every balance is a non-negative
//! 64-bit amount. This contract verifies the proof on-chain using Stellar's
//! native BLS12-381 host functions, then records whether the issuer's attested
//! reserve covers the proven liabilities.
//!
//! A tampered `total` (an issuer lying about liabilities) makes the pairing
//! check fail, so the attestation is cryptographically rejected — it cannot be
//! stored. The verification key is embedded at build time in `vk_data.rs`.

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype,
    crypto::bls12_381::{Fr, G1Affine, G2Affine},
    symbol_short, vec, Address, Bytes, BytesN, Env, U256, Vec,
};

mod vk_data;
#[cfg(test)]
mod test;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    MalformedVerifyingKey = 1,
    ProofRejected = 2,
}

/// A recorded solvency attestation.
#[contracttype]
#[derive(Clone)]
pub struct Attestation {
    pub issuer: Address,
    pub total: u128,   // proven sum of customer liabilities
    pub reserve: u128, // issuer's attested reserves
    pub solvent: bool, // proof valid AND reserve >= total
    pub ledger: u32,
}

#[contracttype]
pub enum Key {
    Latest,
    Count,
}

/// Build a BLS12-381 scalar field element from a u128 amount (big-endian).
fn fr_from_u128(env: &Env, v: u128) -> Fr {
    let mut be = [0u8; 32];
    be[16..].copy_from_slice(&v.to_be_bytes());
    let u = U256::from_be_bytes(env, &Bytes::from_array(env, &be));
    Fr::from_u256(u)
}

/// Reconstruct the embedded verification key.
fn load_vk(env: &Env) -> (G1Affine, G2Affine, G2Affine, G2Affine, Vec<G1Affine>) {
    let alpha = G1Affine::from_array(env, &vk_data::VK_ALPHA);
    let beta = G2Affine::from_array(env, &vk_data::VK_BETA);
    let gamma = G2Affine::from_array(env, &vk_data::VK_GAMMA);
    let delta = G2Affine::from_array(env, &vk_data::VK_DELTA);
    let ic = vec![
        env,
        G1Affine::from_array(env, &vk_data::VK_IC0),
        G1Affine::from_array(env, &vk_data::VK_IC1),
    ];
    (alpha, beta, gamma, delta, ic)
}

#[contract]
pub struct SolvencyContract;

#[contractimpl]
impl SolvencyContract {
    /// Verify a solvency proof and record the attestation.
    ///
    /// Returns `true` if the proof is valid AND `reserve >= total` (solvent),
    /// `false` if the proof is valid but reserves are insufficient (insolvent),
    /// and errors with `ProofRejected` if the proof does not verify (e.g. the
    /// issuer tampered with the published total).
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

        let (alpha, beta, gamma, delta, ic) = load_vk(&env);

        // Single public signal: the claimed total of liabilities.
        let pub_signals = vec![&env, fr_from_u128(&env, total)];
        if pub_signals.len() + 1 != ic.len() {
            return Err(Error::MalformedVerifyingKey);
        }

        let bls = env.crypto().bls12_381();
        let a = G1Affine::from_bytes(proof_a);
        let b = G2Affine::from_bytes(proof_b);
        let c = G1Affine::from_bytes(proof_c);

        // vk_x = ic[0] + sum(pub_signals[i] * ic[i+1])
        let mut vk_x = ic.get(0).unwrap();
        for (s, v) in pub_signals.iter().zip(ic.iter().skip(1)) {
            let prod = bls.g1_mul(&v, &s);
            vk_x = bls.g1_add(&vk_x, &prod);
        }

        // e(-A, B) * e(alpha, beta) * e(vk_x, gamma) * e(C, delta) == 1
        let neg_a = -a;
        let vp1 = vec![&env, neg_a, alpha, vk_x, c];
        let vp2 = vec![&env, b, beta, gamma, delta];
        if !bls.pairing_check(vp1, vp2) {
            return Err(Error::ProofRejected);
        }

        let solvent = reserve >= total;
        let att = Attestation {
            issuer: issuer.clone(),
            total,
            reserve,
            solvent,
            ledger: env.ledger().sequence(),
        };
        env.storage().instance().set(&Key::Latest, &att);
        let count: u32 = env.storage().instance().get(&Key::Count).unwrap_or(0);
        env.storage().instance().set(&Key::Count, &(count + 1));
        env.events()
            .publish((symbol_short!("attest"), issuer), (total, reserve, solvent));
        Ok(solvent)
    }

    /// Latest recorded attestation, if any.
    pub fn latest(env: Env) -> Option<Attestation> {
        env.storage().instance().get(&Key::Latest)
    }

    /// True if the most recent attestation proved solvency.
    pub fn is_solvent(env: Env) -> bool {
        let att: Option<Attestation> = env.storage().instance().get(&Key::Latest);
        att.map(|a| a.solvent).unwrap_or(false)
    }

    /// Total number of attestations recorded.
    pub fn count(env: Env) -> u32 {
        env.storage().instance().get(&Key::Count).unwrap_or(0)
    }
}
