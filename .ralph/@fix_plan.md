# Fix Plan — Solvent (ZK Proof-of-Reserves on Stellar)

## Tasks

- [ ] Task 1: Set up Stellar toolchain + testnet account
  - Acceptance: `stellar` CLI installed, a funded testnet account exists (via Lab/friendbot), `stellar contract build` works on a hello-world Soroban contract.
  - Files: contracts/, Cargo.toml
  - Ref: feed agent skills.stellar.org first; https://developers.stellar.org/docs/tools/cli

- [ ] Task 2: MILD stepping stone — proof-of-funds range proof end-to-end
  - Acceptance: Circom circuit proves "balance >= X" without revealing balance; snarkjs generates a Groth16 proof; a Soroban `groth16_verifier` contract on testnet returns TRUE for valid, FALSE for invalid. Proves the whole toolchain.
  - Files: circuits/range.circom, contracts/verifier/, scripts/prove.js
  - Ref: fork Circom/Groth16 pipeline from ~/Projects/privacy-bridge; study NethermindEth/stellar-private-payments

- [ ] Task 3: TARGET circuit — Merkle-sum solvency proof
  - Acceptance: Circom circuit takes N private customer balances, proves Σ = published total AND every balance >= 0; snarkjs proof verifies locally.
  - Files: circuits/solvency.circom, circuits/inputs/

- [ ] Task 4: Soroban solvency verifier contract (Phase 1 Gate)
  - Acceptance: Deployed testnet contract verifies the Groth16 solvency proof and returns verified=true; rejects a tampered published total (returns false / reverts). BOTH branches demonstrated on-chain. THIS IS THE PHASE 1 GATE.
  - Files: contracts/solvency-verifier/src/lib.rs

- [ ] Task 5: Customer inclusion proof
  - Acceptance: a customer can be given a Merkle inclusion proof and verify their balance is inside the attested total.
  - Files: scripts/inclusion.js

- [ ] Task 6: Minimal issuer + customer frontend
  - Acceptance: issuer uploads balances → proof generated client-side (WASM, secrets never leave device) → contract verifies → public sees SOLVENT ✅ with no balances exposed; customer verifies inclusion.
  - Files: web/

## Completed
(builder fills this in)
