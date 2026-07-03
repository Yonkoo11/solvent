# Contributing to Solvent

Thanks for looking! This started as a hackathon build (Stellar Hacks: Real-World ZK).

## Dev setup
See the "Reproduce it" section of the README. You need Rust (+ `wasm32v1-none`), circom 2.x,
Node 18+, and the `stellar` CLI.

## Good first issues
- Swap the fixed N=8 sum for a Merkle-sum tree so customers can verify their own inclusion.
- Use Stellar's native Poseidon host function for the tree hashing instead of an off-chain hash.
- Add a view-key / selective-disclosure path so an auditor can reconstruct detail.
- Replace the toy trusted setup with a real multi-party ceremony.

## Ground rules
- Keep the ZK load-bearing — no shortcuts that make the proof cosmetic.
- Be honest in the README about what is real vs mocked.
- Never commit secrets, `.env`, or keystores (see SECURITY.md).
