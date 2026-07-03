#![cfg(test)]
extern crate std;

use ark_serialize::CanonicalSerialize;
use core::str::FromStr;
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env};

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

// ================= BN254 =================
#[test]
fn bn254_verifies_and_marks_solvent() {
    let env = Env::default();
    let c = client(&env);
    let proof = read("bn254/proof.json");
    let total: u128 = read("bn254/public.json")[0].as_str().unwrap().parse().unwrap();
    let (a, b, cc) = (bn_g1(&env, &proof["pi_a"]), bn_g2(&env, &proof["pi_b"]), bn_g1(&env, &proof["pi_c"]));
    let issuer = Address::generate(&env);

    assert_eq!(c.attest_bn254(&issuer, &total, &(total + 500), &a, &b, &cc), true);
    assert_eq!(c.latest(&issuer).unwrap().curve, soroban_sdk::symbol_short!("BN254"));
    // tampered total on BN254 path is rejected too
    assert!(c.try_attest_bn254(&issuer, &(total - 1), &(total + 500), &a, &b, &cc).is_err());
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
