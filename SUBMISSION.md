# Solvent — DoraHacks submission (copy field-by-field)

Hackathon: Stellar Hacks: Real-World ZK. Fill the tabs in order: Profile, Details, Team, Contact, Submission.

================================================================
## TAB 1 — PROFILE
================================================================

### Vision  (the problem this solves)  [max 256 chars — this is 246]
Proof of reserves today is a spreadsheet you must trust. Issuers say they are backed, but you take their word for it. Proving it honestly would expose every customer balance. Solvent proves solvency without trusting them or revealing any balance.

### BUIDL logo
Upload: ~/Projects/solvent/video/logo.png  (480x480)

### Category
Crypto / Web3

### Is this BUIDL an AI Agent?
No

### GitHub / Gitlab / Bitbucket
https://github.com/Yonkoo11/solvent

================================================================
## TAB 2 — DETAILS
================================================================

### Full description
Solvent lets a stablecoin or real-world-asset issuer prove on-chain that its reserves cover every customer liability, without revealing a single account balance. A Soroban smart contract verifies a zero-knowledge proof and records the verdict: SOLVENT or INSOLVENT.

The zero-knowledge is load-bearing, not decoration. A Circom circuit proves the private customer balances sum to a public total, and that none of them is a negative liability sneaked in to shrink the number (the Mt. Gox trick). Because the total is baked into the proof, lying about it makes the check fail, so the contract can never record a lie.

Two of Stellar's native ZK primitives are load-bearing. The contract can verify the same solvency proof over BLS12-381 (Protocol 25 host functions) and over BN254 (the Protocol 26 host functions), both proven live on testnet. Attestations are stored per issuer, so one issuer can never overwrite or grief another's published verdict.

It runs live on Stellar testnet. In the browser you enter balances, a real proof is built locally (your balances never leave the page), and the live board reads the on-chain verdict.

How it works:
1. Prove privately (off-chain, Circom): the circuit proves the sum plus range checks. Balances never leave the issuer's machine.
2. Verify on Stellar (Soroban): the contract verifies the Groth16 proof with native BLS12-381, then checks reserve is at least the proven total.
3. Trust the verdict (public): anyone reads SOLVENT or INSOLVENT from the chain.

Proven on-chain (live on testnet):
- Valid proof, reserve covers liabilities: returns true, SOLVENT (recorded).
- Valid proof, reserve below liabilities: returns false, INSOLVENT (recorded).
- Tampered total: rejected by the contract, nothing recorded.

Honest note: the reserve figure is a signed issuer stand-in for a real custodian feed; the zero-knowledge part is real and verified on-chain. Testnet only, not audited.

### Live demo / website
https://yonkoo11.github.io/solvent/

### Demo video
Upload the file: ~/Projects/solvent/video/solvent-demo.mp4

### Stellar contract (testnet)
CCRAJEUHMJUUVJX3SJZWUPGJKRF43ZXDETWFNCSAKND3FPAI33QXJZBE

### Tech stack / tags
Zero Knowledge, ZK, Circom, snarkjs, Groth16, BLS12-381, BN254, Soroban, Stellar, Proof of Reserves, Stablecoins, RWA

================================================================
## TAB 3 — TEAM
================================================================

Solo builder. (Add your name / handle: Yonkoo11.)

================================================================
## TAB 4 — CONTACT
================================================================

GitHub: https://github.com/Yonkoo11
(Add your email / Telegram / Discord as the form asks.)

================================================================
## TAB 5 — SUBMISSION
================================================================

### Requirements checklist (this hackathon)
- Open-source repo with clear README: https://github.com/Yonkoo11/solvent
- Demo video (2 to 3 min): video/solvent-demo.mp4 (uploaded on the Details tab)
- ZK used in a meaningful, load-bearing way: yes, the product is a verifier contract
- Touches Stellar: yes, Groth16 proof verified in a Soroban testnet contract

### One-line summary (if asked)
Solvent: ZK proof-of-reserves for Stellar issuers. Prove reserves cover liabilities without revealing any balance.
