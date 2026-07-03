#![cfg(test)]
extern crate std;

use ark_bls12_381::{Fq, Fq2};
use ark_serialize::CanonicalSerialize;
use core::str::FromStr;
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env};

use crate::{SolvencyContract, SolvencyContractClient};

// ---- helpers: snarkjs decimal-string coords -> uncompressed bytes ----

fn g1(env: &Env, coords: &serde_json::Value) -> BytesN<96> {
    let x = Fq::from_str(coords[0].as_str().unwrap()).unwrap();
    let y = Fq::from_str(coords[1].as_str().unwrap()).unwrap();
    let p = ark_bls12_381::G1Affine::new(x, y);
    let mut buf = [0u8; 96];
    p.serialize_uncompressed(&mut buf[..]).unwrap();
    BytesN::from_array(env, &buf)
}

fn g2(env: &Env, coords: &serde_json::Value) -> BytesN<192> {
    let x = Fq2::new(
        Fq::from_str(coords[0][0].as_str().unwrap()).unwrap(),
        Fq::from_str(coords[0][1].as_str().unwrap()).unwrap(),
    );
    let y = Fq2::new(
        Fq::from_str(coords[1][0].as_str().unwrap()).unwrap(),
        Fq::from_str(coords[1][1].as_str().unwrap()).unwrap(),
    );
    let p = ark_bls12_381::G2Affine::new(x, y);
    let mut buf = [0u8; 192];
    p.serialize_uncompressed(&mut buf[..]).unwrap();
    BytesN::from_array(env, &buf)
}

fn load() -> (serde_json::Value, u128) {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../build");
    let proof: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(base.join("proof.json")).unwrap()).unwrap();
    let public: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(base.join("public.json")).unwrap()).unwrap();
    let total: u128 = public[0].as_str().unwrap().parse().unwrap();
    (proof, total)
}

fn setup(env: &Env) -> (SolvencyContractClient, Address, BytesN<96>, BytesN<192>, BytesN<96>, u128) {
    env.mock_all_auths();
    let (proof, total) = load();
    let a = g1(env, &proof["pi_a"]);
    let b = g2(env, &proof["pi_b"]);
    let c = g1(env, &proof["pi_c"]);
    let id = env.register(SolvencyContract, ());
    let client = SolvencyContractClient::new(env, &id);
    let issuer = Address::generate(env);
    (client, issuer, a, b, c, total)
}

#[test]
fn valid_proof_marks_solvent() {
    let env = Env::default();
    let (client, issuer, a, b, c, total) = setup(&env);
    // reserve fully covers proven liabilities -> solvent
    let res = client.attest(&issuer, &total, &(total + 1_000), &a, &b, &c);
    assert_eq!(res, true, "valid proof + adequate reserve should be solvent");
    assert_eq!(client.is_solvent(), true);
    assert_eq!(client.count(), 1);
    let att = client.latest().unwrap();
    assert_eq!(att.total, total);
}

#[test]
fn valid_proof_insufficient_reserve_is_insolvent() {
    let env = Env::default();
    let (client, issuer, a, b, c, total) = setup(&env);
    // proof valid, but reserve below liabilities -> insolvent (recorded, not rejected)
    let res = client.attest(&issuer, &total, &(total - 1), &a, &b, &c);
    assert_eq!(res, false, "valid proof but short reserve should be insolvent");
    assert_eq!(client.is_solvent(), false);
}

#[test]
fn tampered_total_is_rejected() {
    let env = Env::default();
    let (client, issuer, a, b, c, total) = setup(&env);
    // issuer lies: claims a smaller total than the proof attests -> pairing fails
    let bogus = total - 1;
    let r = client.try_attest(&issuer, &bogus, &(bogus + 1_000), &a, &b, &c);
    assert!(r.is_err(), "tampered total must be cryptographically rejected");
    // nothing recorded
    assert_eq!(client.count(), 0);
}
