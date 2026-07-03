# Solvent — prove your reserves, not your books

**ZK proof-of-reserves for Stellar issuers.** A stablecoin or RWA issuer proves on-chain that its
reserves cover every customer liability — **without revealing a single account balance** — and a
Soroban smart contract verifies the zero-knowledge proof using Stellar's native BLS12-381 host
functions. Lying about the total is cryptographically impossible: a tampered figure makes the
pairing check fail, so it can never be recorded as a valid attestation.

Built for **Stellar Hacks: Real-World ZK**.

- **Live demo:** https://yonkoo11.github.io/solvent/
- **Contract (testnet):** [`CCRAJEUHMJUUVJX3SJZWUPGJKRF43ZXDETWFNCSAKND3FPAI33QXJZBE`](https://stellar.expert/explorer/testnet/contract/CCRAJEUHMJUUVJX3SJZWUPGJKRF43ZXDETWFNCSAKND3FPAI33QXJZBE)

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
**two** of Stellar's native ZK primitives, both proven live on testnet:

- `attest` verifies over **BLS12-381** — `env.crypto().bls12_381().pairing_check(...)` (Protocol 25 / CAP-0059)
- `attest_bn254` verifies over **BN254** — `env.crypto().bn254().pairing_check(...)` (Protocol 26 / CAP-0074)

The contract then checks the issuer's attested reserve against the *proven* total and records
**SOLVENT** or **INSOLVENT**. Attestations are stored **per issuer**, so one issuer can never
overwrite or grief another's published verdict.

Because the total `T` is a public input to the proof, changing it on the contract call changes the
field element the pairing is computed against — so a tampered total is rejected by the cryptography,
not by a trusted server.

## Proven on-chain (Stellar testnet)

All three branches were exercised live against the deployed contract:

| Case | Input | Result |
|---|---|---|
| Valid proof, reserve ≥ liabilities | total 1500, reserve 5000 | `true` → **SOLVENT** (recorded) |
| Valid proof, reserve < liabilities | total 1500, reserve 1000 | `false` → **INSOLVENT** (recorded) |
| **Tampered** total | proof for 1500, claim 1499 | `Error(Contract, #2)` **ProofRejected** (nothing recorded) |

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
./scripts/build-circuit.sh                 # circuit -> Groth16 proof (verifies off-chain)
( cd scripts/converter && cargo run )      # snarkjs JSON -> embedded VK + invoke args
cd contracts/solvency && cargo test        # runs the REAL proof through the contract (3/3)
stellar contract build                     # -> target/wasm32v1-none/release/solvency_verifier.wasm
```

`cargo test` runs your real proof through the actual Soroban BLS12-381 host functions — it is
genuine cryptographic verification, not a mock. To deploy + attest on testnet, see
[`scripts/attest.sh`](scripts/attest.sh).

## Honest disclosures (work-in-progress, per hackathon guidance)

- The **reserve** figure is a signed issuer attestation — in this demo it is a value the issuer
  passes in (a stand-in for a real custodian feed). The zero-knowledge part (that the published
  total equals the sum of the hidden balances, with no negative-balance cheat) is real and verified
  on-chain. Wiring a real custodian attestation is the obvious next step.
- The circuit is fixed at **N = 8** balances for the demo; it is parameterizable. A production
  version would use a Merkle-sum tree so each customer can verify their own inclusion, and would use
  Stellar's native Poseidon host function for the tree hashing.
- Testnet only. The contract has **not** been audited. Do not use with real assets.
- The trusted setup here is a single-contributor toy ceremony. Production needs a real MPC ceremony.

## License

MIT — see [LICENSE](LICENSE).
