# Solvent — DoraHacks submission

Paste this into the DoraHacks BUIDL form for **Stellar Hacks: Real-World ZK**.

---

## Name
Solvent

## Tagline (one line)
Prove your reserves, not your books: ZK proof-of-reserves for Stellar issuers.

## Links
- **Live demo:** https://yonkoo11.github.io/solvent/
- **Open-source repo:** https://github.com/Yonkoo11/solvent
- **Demo video:** video/solvent-demo.mp4 in the repo (upload to the form / YouTube unlisted)
- **Contract (Stellar testnet):** `CC33DWJZK2XQ7KWKBDJK2KBJSQNWUPG3DSGCHNMMOAIGC3VTAXZQLHDJ`
  - https://stellar.expert/explorer/testnet/contract/CC33DWJZK2XQ7KWKBDJK2KBJSQNWUPG3DSGCHNMMOAIGC3VTAXZQLHDJ

## Tags
Zero Knowledge, ZK, Groth16, BLS12-381, Circom, Soroban, Stellar, Proof of Reserves, Stablecoins, RWA

---

## What it is

A stablecoin or RWA issuer on Stellar proves on-chain that its reserves cover every customer
liability, **without revealing a single account balance**. A Soroban smart contract verifies a
zero-knowledge proof and records the verdict: SOLVENT or INSOLVENT. Lying about the total is
cryptographically impossible, a tampered figure fails the pairing check and can never be recorded.

Exchanges publish "proof of reserves" that is really just a spreadsheet you have to trust. Solvent
makes it a cryptographic fact, verified by the chain the assets live on.

## What the zero-knowledge is doing (load-bearing)

The whole product is a verifier. Remove the proof and there is nothing left.

An issuer holds private per-customer balances. A **Circom** circuit proves two things at once:
1. the balances sum to a single **public total** (the issuer's liabilities), and
2. every balance is a non-negative 64-bit amount, a range check that blocks the classic Mt. Gox
   trick of inserting a "negative" liability (a field element near the curve order) to shrink the
   reported total.

The proof is **Groth16 over BLS12-381**. Because the total is a public input, changing it on the
contract call changes the field element the pairing is computed against, so the cryptography itself
rejects a lie. No trusted server involved.

## Stellar integration

The verifier is a **Soroban smart contract**. It checks the proof using Stellar's **native
BLS12-381 host functions** (CAP-0059, `env.crypto().bls12_381().pairing_check`), then confirms the
attested reserve covers the proven total and stores the attestation on-chain. Protocol 25/26 make
this verification cheap enough to run as a routine contract call.

## How it works

1. **Prove privately (off-chain, Circom):** the circuit proves the sum + range checks. Balances never
   leave the issuer's machine. The website does this client-side in WebAssembly.
2. **Verify on Stellar (Soroban):** the contract verifies the Groth16 proof with native BLS12-381,
   then checks reserve >= proven total.
3. **Trust the verdict (public):** anyone reads SOLVENT / INSOLVENT from the chain.

## Proven on-chain (all three cases, live on testnet)

| Case | Result |
|---|---|
| Valid proof, reserve >= liabilities | `true` SOLVENT (recorded) |
| Valid proof, reserve < liabilities | `false` INSOLVENT (recorded) |
| Tampered total | `Error(Contract, #2)` ProofRejected (nothing recorded) |

## Tech stack

Circom 2 + snarkjs (Groth16, BLS12-381), arkworks (proof to on-chain byte conversion), Soroban SDK
25.1.0 (Rust, wasm32v1-none), Stellar CLI, static web frontend (snarkjs in-browser + Stellar SDK
reading live contract state).

## Try it

Open https://yonkoo11.github.io/solvent/ , enter eight customer balances, and click Generate. The
browser builds a real proof and shows the balances stay hidden while the total is proven. The live
board reads the current attestation straight from the testnet contract.

## Honest disclosures (per hackathon guidance)

- The **reserve** figure is a signed issuer attestation, in this demo a value the issuer passes in
  (a stand-in for a real custodian feed). The zero-knowledge part (total = sum of hidden balances,
  no negative-balance cheat) is real and verified on-chain. A real custodian attestation is the next
  step.
- The circuit is fixed at 8 balances for the demo; it is parameterizable. A production version uses a
  Merkle-sum tree so each customer can verify their own inclusion, hashed with Stellar's native
  Poseidon host function.
- Testnet only. The contract is not audited. Do not use with real assets. The trusted setup here is a
  single-contributor toy ceremony; production needs a real multi-party ceremony.

## Submission requirements met

- Open-source repo with clear README: yes
- Demo video (2 to 3 min): yes
- ZK used in a meaningful, load-bearing way: yes (the product is a verifier)
- Touches Stellar: yes (Groth16 proof verified in a Soroban testnet contract)
