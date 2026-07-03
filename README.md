# Solvent — prove your reserves, not your books

**ZK proof-of-reserves for Stellar issuers.** A stablecoin or RWA issuer proves on-chain that its
reserves cover every customer liability — **without revealing a single account balance** — and a
Soroban smart contract verifies the zero-knowledge proof using Stellar's native BLS12-381 host
functions. Lying about the total is cryptographically impossible: a tampered figure makes the
pairing check fail, so it can never be recorded as a valid attestation.

Built for **Stellar Hacks: Real-World ZK**.

- **Live demo:** https://yonkoo11.github.io/solvent/
- **Contract (testnet):** [`CCVRCYVNKJ2OZPWJC7PDETHBTE5R5EJPCX5DFZS5PRKX7FHPLLQK2HNV`](https://stellar.expert/explorer/testnet/contract/CCVRCYVNKJ2OZPWJC7PDETHBTE5R5EJPCX5DFZS5PRKX7FHPLLQK2HNV)

---

## What the ZK is actually doing (it's load-bearing)

The whole product is a verifier. Remove the zero-knowledge proof and there is nothing left.

An issuer holds private per-customer balances `b[0..N]`. They want to publish a single number —
their total liabilities `T` — and let anyone confirm it is honest, without exposing the individual
balances. A [Circom circuit](circuits/solvency.circom) proves two things at once:

1. **`sum(b) == T`** — the published total really is the sum of the hidden balances.
2. **each `b[i]` is a non-negative 64-bit amount** — a range check that blocks the classic
   Mt. Gox trick of inserting a "negative" liability (a field element near the curve order) to
   shrink the reported total.

The proof (Groth16) is verified inside the [Soroban contract](contracts/solvency/src/lib.rs) using
**three** of Stellar's native ZK primitives, all proven live on testnet:

- `attest` verifies over **BLS12-381** — `env.crypto().bls12_381().pairing_check(...)` (Protocol 25 / CAP-0059)
- `attest_bn254` verifies over **BN254** — `env.crypto().bn254().pairing_check(...)` (Protocol 26 / CAP-0074)
- `verify_inclusion` uses the native **Poseidon** host function — `env.crypto_hazmat().poseidon_permutation(...)` (Protocol 25 / CAP-0075) — to check a customer's Merkle inclusion against the **proof-bound** root

The BN254 path uses a second circuit ([`solvency_bound.circom`](circuits/solvency_bound.circom)) that
also computes the **Poseidon Merkle root of the same balances in-circuit** and publishes it. So the
root a customer checks their inclusion against is the *same* set of balances the total was proven
from. Tampering the root fails the proof; a fake customer leaf fails `verify_inclusion`. Both checked
live on-chain.

The contract then checks the issuer's attested reserve against the *proven* total and records
**SOLVENT** or **INSOLVENT**. Attestations are stored **per issuer**, so one issuer can never
overwrite or grief another's published verdict.

The Poseidon wiring is not trusted blind: [`scripts/poseidon-gen.mjs`](scripts/poseidon-gen.mjs)
validates circomlib's standard constants against the canonical `poseidon([1,2])` vector before
emitting them, and the `poseidon_matches_reference` test confirms the **on-chain** host function
reproduces that same value. `hash2(1,2)` on the live contract returns the canonical
`0x115cc0f5…` result.

Because the total `T` is a public input to the proof, changing it on the contract call changes the
field element the pairing is computed against — so a tampered total is rejected by the cryptography,
not by a trusted server.

## Proven on-chain (Stellar testnet)

All exercised live against the deployed contract:

| Case | Result |
|---|---|
| Valid proof (BLS12-381), reserve ≥ liabilities | `true` → **SOLVENT** (recorded) |
| Valid proof (BN254), reserve ≥ liabilities | `true` → **SOLVENT** (recorded) |
| Valid proof, reserve < liabilities | `false` → **INSOLVENT** (recorded) |
| **Tampered** total or root (BN254) | `Error(Contract, #2)` **ProofRejected** (nothing recorded) |
| Poseidon `hash2(1,2)` on-chain | `0x115cc0f5…` — the canonical `poseidon([1,2])` vector |
| `verify_inclusion` real customer leaf | `true` (verified against the proof-bound root) |
| `verify_inclusion` fake leaf | `false` |

## Repo layout

```
circuits/solvency.circom     the ZK circuit (sum + per-balance range check)
scripts/build-circuit.sh     compile + trusted setup + prove + off-chain verify (BLS12-381)
scripts/converter/           arkworks tool: snarkjs JSON -> uncompressed on-chain bytes
scripts/attest.sh            issuer tool: prove any balances -> submit attestation to testnet
contracts/solvency/          Soroban verifier + solvency logic (soroban-sdk 25.1.0)
docs/                         static dashboard: live on-chain board + in-browser prover
```

## Reproduce it

Prereqs: Rust + `wasm32v1-none`, [`circom`](https://docs.circom.io) 2.x, Node 18+,
[`stellar` CLI](https://developers.stellar.org/docs/tools/cli) 27+.

```bash
npm install
./scripts/build-circuit.sh                              # BLS12-381 Groth16 proof (verifies off-chain)
( cd scripts/converter && cargo run --bin convert )     # snarkjs JSON -> embedded BLS12-381 VK + args
node scripts/poseidon-gen.mjs                           # validate + emit Poseidon constants
cd contracts/solvency && cargo test                     # 5/5: BLS + BN254 + Poseidon + inclusion + per-issuer
stellar contract build                                  # -> target/wasm32v1-none/release/solvency_verifier.wasm
```

`cargo test` runs your real proof through the actual Soroban host functions (BLS12-381, BN254, and
Poseidon) — genuine cryptographic verification, not a mock. To deploy + attest on testnet, see
[`scripts/attest.sh`](scripts/attest.sh).

## Honest disclosures (work-in-progress, per hackathon guidance)

- The **reserve** figure is a signed issuer attestation — in this demo it is a value the issuer
  passes in (a stand-in for a real custodian feed). The zero-knowledge part (that the published
  total equals the sum of the hidden balances, with no negative-balance cheat) is real and verified
  on-chain. Wiring a real custodian attestation is the obvious next step.
- **Poseidon Merkle inclusion is bound to the ZK proof** (BN254 path): the bound circuit computes the
  Poseidon root of the same balances in-circuit and publishes it, so `verify_inclusion` checks a
  customer against the exact set the total was proven from. A real leaf verifies and a fake leaf fails,
  live on-chain. (The BLS12-381 `attest` path proves the sum only and stores `root = 0`.)
- The circuit is fixed at **N = 8** balances for the demo; it is parameterizable.
- Testnet only. The contract has **not** been audited. Do not use with real assets.
- The trusted setup here is a single-contributor toy ceremony. Production needs a real MPC ceremony.

## License

MIT — see [LICENSE](LICENSE).
