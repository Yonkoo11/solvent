#![cfg(test)]
extern crate std;

use ark_serialize::CanonicalSerialize;
use core::str::FromStr;
use soroban_sdk::{testutils::Address as _, Address, Bytes, BytesN, Env, U256};

use crate::{SolvencyContract, SolvencyContractClient};

// ---- BLS12-381 helpers (snarkjs decimal coords -> uncompressed bytes) ----
fn bls_g1(env: &Env, c: &serde_json::Value) -> BytesN<96> {
    use ark_bls12_381::{Fq, G1Affine};
    let p = G1Affine::new(
        Fq::from_str(c[0].as_str().unwrap()).unwrap(),
        Fq::from_str(c[1].as_str().unwrap()).unwrap(),
    );
    let mut b = [0u8; 96];
    p.serialize_uncompressed(&mut b[..]).unwrap();
    BytesN::from_array(env, &b)
}
fn bls_g2(env: &Env, c: &serde_json::Value) -> BytesN<192> {
    use ark_bls12_381::{Fq, Fq2, G2Affine};
    let x = Fq2::new(Fq::from_str(c[0][0].as_str().unwrap()).unwrap(), Fq::from_str(c[0][1].as_str().unwrap()).unwrap());
    let y = Fq2::new(Fq::from_str(c[1][0].as_str().unwrap()).unwrap(), Fq::from_str(c[1][1].as_str().unwrap()).unwrap());
    let p = G2Affine::new(x, y);
    let mut b = [0u8; 192];
    p.serialize_uncompressed(&mut b[..]).unwrap();
    BytesN::from_array(env, &b)
}
// ---- BN254 helpers (Ethereum-compatible: 32-byte BE, G2 in c1,c0 order) ----
fn bn_fq_be(s: &str) -> [u8; 32] {
    use ark_bn254::Fq;
    use ark_ff::{BigInteger, PrimeField};
    let be = Fq::from_str(s).unwrap().into_bigint().to_bytes_be();
    let mut out = [0u8; 32];
    out[32 - be.len()..].copy_from_slice(&be);
    out
}
fn bn_g1(env: &Env, c: &serde_json::Value) -> BytesN<64> {
    let mut b = [0u8; 64];
    b[..32].copy_from_slice(&bn_fq_be(c[0].as_str().unwrap()));
    b[32..].copy_from_slice(&bn_fq_be(c[1].as_str().unwrap()));
    BytesN::from_array(env, &b)
}
fn bn_g2(env: &Env, c: &serde_json::Value) -> BytesN<128> {
    let mut b = [0u8; 128];
    b[0..32].copy_from_slice(&bn_fq_be(c[0][1].as_str().unwrap()));
    b[32..64].copy_from_slice(&bn_fq_be(c[0][0].as_str().unwrap()));
    b[64..96].copy_from_slice(&bn_fq_be(c[1][1].as_str().unwrap()));
    b[96..128].copy_from_slice(&bn_fq_be(c[1][0].as_str().unwrap()));
    BytesN::from_array(env, &b)
}
// BN254 scalar-field decimal string -> U256 (for the Merkle root public signal)
fn bn_fr_u256(env: &Env, s: &str) -> U256 {
    use ark_bn254::Fr;
    use ark_ff::{BigInteger, PrimeField};
    let be = Fr::from_str(s).unwrap().into_bigint().to_bytes_be();
    let mut out = [0u8; 32];
    out[32 - be.len()..].copy_from_slice(&be);
    U256::from_be_bytes(env, &Bytes::from_array(env, &out))
}

fn read(path: &str) -> serde_json::Value {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../build");
    serde_json::from_str(&std::fs::read_to_string(base.join(path)).unwrap()).unwrap()
}

fn client(env: &Env) -> SolvencyContractClient {
    env.mock_all_auths();
    SolvencyContractClient::new(env, &env.register(SolvencyContract, ()))
}

// ================= BLS12-381 =================
#[test]
fn bls_solvent_and_insolvent_and_tampered() {
    let env = Env::default();
    let c = client(&env);
    let proof = read("proof.json");
    let total: u128 = read("public.json")[0].as_str().unwrap().parse().unwrap();
    let (a, b, cc) = (bls_g1(&env, &proof["pi_a"]), bls_g2(&env, &proof["pi_b"]), bls_g1(&env, &proof["pi_c"]));
    let issuer = Address::generate(&env);

    assert_eq!(c.attest(&issuer, &total, &(total + 1000), &a, &b, &cc), true);
    assert_eq!(c.is_solvent(&issuer), true);
    assert_eq!(c.latest(&issuer).unwrap().curve, soroban_sdk::symbol_short!("BLS12_381"));

    assert_eq!(c.attest(&issuer, &total, &(total - 1), &a, &b, &cc), false);
    assert_eq!(c.is_solvent(&issuer), false);

    let bogus = total - 1;
    assert!(c.try_attest(&issuer, &bogus, &(bogus + 1000), &a, &b, &cc).is_err());
}

// ================= BN254 (bound circuit: proves sum AND Merkle root) =================
#[test]
fn bn254_bound_verifies_and_marks_solvent() {
    let env = Env::default();
    let c = client(&env);
    let proof = read("bound/proof.json");
    let public = read("bound/public.json"); // [root, total]
    let root = bn_fr_u256(&env, public[0].as_str().unwrap());
    let total: u128 = public[1].as_str().unwrap().parse().unwrap();
    let (a, b, cc) = (bn_g1(&env, &proof["pi_a"]), bn_g2(&env, &proof["pi_b"]), bn_g1(&env, &proof["pi_c"]));
    let issuer = Address::generate(&env);

    assert_eq!(c.attest_bn254(&issuer, &total, &(total + 500), &root, &a, &b, &cc), true);
    let att = c.latest(&issuer).unwrap();
    assert_eq!(att.curve, soroban_sdk::symbol_short!("BN254"));
    assert_eq!(att.root, root, "the proof-bound Merkle root must be stored");
    // tampered total is rejected
    assert!(c.try_attest_bn254(&issuer, &(total - 1), &(total + 500), &root, &a, &b, &cc).is_err());
    // tampered root is rejected (proof binds root to the balances)
    let bad_root = bn_fr_u256(&env, "12345");
    assert!(c.try_attest_bn254(&issuer, &total, &(total + 500), &bad_root, &a, &b, &cc).is_err());
}

// ============ Poseidon host-function gate (must pass before use) ============
#[test]
fn poseidon_matches_reference() {
    let env = Env::default();
    let c = client(&env);
    let got = c.hash2(&U256::from_u32(&env, 1), &U256::from_u32(&env, 2));
    let expected = U256::from_be_bytes(&env, &Bytes::from_array(&env, &crate::poseidon_params::REF_1_2));
    assert_eq!(got, expected, "on-chain Poseidon must match circomlib poseidon([1,2])");
}

// ==== Bound Merkle inclusion: customer verifies against the PROOF-BOUND root ====
// Uses the same balances the bound circuit proved: [1200,800,400,2500,100,900,1100,1000].
#[test]
fn inclusion_against_bound_root() {
    let env = Env::default();
    let c = client(&env);
    // First, land a real bound attestation so the issuer's stored root is the
    // one the circuit proved.
    let proof = read("bound/proof.json");
    let public = read("bound/public.json");
    let root = bn_fr_u256(&env, public[0].as_str().unwrap());
    let total: u128 = public[1].as_str().unwrap().parse().unwrap();
    let (a, b, cc) = (bn_g1(&env, &proof["pi_a"]), bn_g2(&env, &proof["pi_b"]), bn_g1(&env, &proof["pi_c"]));
    let issuer = Address::generate(&env);
    assert_eq!(c.attest_bn254(&issuer, &total, &(total + 500), &root, &a, &b, &cc), true);

    // Build customer 0's inclusion path (leaf = balance 1200) using the same
    // Poseidon the contract uses.
    let u = |n: u32| U256::from_u32(&env, n);
    let l1_1 = c.hash2(&u(400), &u(2500));
    let l2_1 = c.hash2(&c.hash2(&u(100), &u(900)), &c.hash2(&u(1100), &u(1000)));
    let mut path = soroban_sdk::Vec::new(&env);
    path.push_back((u(800), false)); // sibling 800 on the right
    path.push_back((l1_1, false));
    path.push_back((l2_1, false));

    assert_eq!(c.verify_inclusion(&issuer, &u(1200), &path), true, "real leaf must verify against bound root");
    assert_eq!(c.verify_inclusion(&issuer, &u(9999), &path), false, "fake leaf must not verify");
    // an issuer with no attestation verifies nothing
    let stranger = Address::generate(&env);
    assert_eq!(c.verify_inclusion(&stranger, &u(1200), &path), false);
}

// ================= per-issuer isolation =================
#[test]
fn issuers_do_not_collide() {
    let env = Env::default();
    let c = client(&env);
    let proof = read("proof.json");
    let total: u128 = read("public.json")[0].as_str().unwrap().parse().unwrap();
    let (a, b, cc) = (bls_g1(&env, &proof["pi_a"]), bls_g2(&env, &proof["pi_b"]), bls_g1(&env, &proof["pi_c"]));
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);

    c.attest(&alice, &total, &(total + 1), &a, &b, &cc); // alice solvent
    c.attest(&bob, &total, &(total - 1), &a, &b, &cc); // bob insolvent
    assert_eq!(c.is_solvent(&alice), true);
    assert_eq!(c.is_solvent(&bob), false);
    assert_eq!(c.count(), 2);
}
