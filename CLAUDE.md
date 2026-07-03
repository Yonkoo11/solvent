# Solvent — ZK Proof-of-Reserves on Stellar

## Vibecoder Mode
- Never say: branch, commit, merge, PR, push, pull, HEAD, diff, npm, deploy, lint, env var. Say: version, save point, combine changes, publish, update, latest, changes, install, check code.
- Never show raw terminal output or error messages. Summarize in one sentence; say what happened + what you're doing to fix it.
- Auto-save after every completed task (git add specific files + commit). Never ask.
- Fix failing tests silently. Describe changes by what the user SEES, not files.
- After each task update ai/progress.md with a "What Changed (Plain English)" section.

## Phase 1 Gate (BUILD THIS FIRST — nothing else until it passes)
Core Action: Issuer submits private customer balances → Circom circuit proves Σ = published total with every balance >= 0 → Soroban testnet contract verifies the Groth16 proof and returns verified=true.
Success Test (binary): testnet contract returns TRUE for a valid proof AND FALSE/rejects a tampered total — both shown on-chain.
Build order: Phase 1 (verify a proof on testnet) → Phase 2 (Merkle-sum solvency circuit) → Phase 3 (inclusion proofs + frontend) → Phase 4 (polish). Do the on-chain verify BEFORE any CSS.

## Hackathon Context
- Stellar Hacks: Real-World ZK (SDF via DoraHacks). Deadline (raw): 2026-07-03 18:00. Prize: $10K single open track.
- ZK MUST be load-bearing (page requirement). Real-world money use cases (stablecoins/RWA/settlement) especially welcome.
- Stack: Circom + snarkjs Groth16 → Soroban groth16_verifier contract (Protocol 25/26 native BN254 + Poseidon host funcs).
- Feed the agent skills.stellar.org (ZK Proofs skill) FIRST. Study NethermindEth/stellar-private-payments + soroban-examples/groth16_verifier. Reuse Circom/Groth16 pipeline from ~/Projects/privacy-bridge.
- Full context: ai/memory.md. Sponsor depth: ai/sponsor-integration.md. Research base: ~/Projects/IDEAS-SUMMARY.md.

## Sponsor Depth Targets (Stellar — target 5/5)
- Soroban Groth16 verifier contract on testnet is the product (load-bearing).
- Use native Poseidon host function for Merkle-sum hashing (not a Rust reimpl).
- Client-side (WASM) proof gen so balances never leave the device.
- HONESTY: the reserve figure is a mock/signed attestation for the demo — disclose it plainly in the README.

## SECURITY — KEYS NEVER IN REPO OR CONTEXT (BLOCKING)

The deployer + operator + RPC keys live ONLY in `~/.zshenv`. Hard rules:

- **NEVER read `~/.zshenv`, `~/.zshrc`, `~/.zprofile`, `~/.bashrc`, `~/.bash_profile`, `~/.netrc`, `~/.npmrc`, `~/.git-credentials`, SSH keys, `*.key`, `*.pem`, or any `keystore/*` file.** Not `Read`, not `cat`, not `head`, not `grep -v`. Project + global hooks block these.
- **NEVER print, echo, or log key values.** `echo $KEY`, `print(os.getenv("KEY"))`, `vm.toString(pk)`, `console.log(process.env.KEY)` are banned.
- **NEVER commit `.env*`, `*.key`, `*.pem`, `keystore/`, `secrets/`** — covered by `.gitignore`. Verify `git diff --cached` before every save point.
- **NEVER use `git add -A` for the first save point in a new project.** Add by explicit file name.
- **Foundry deploys use `vm.envUint("DEPLOYER_PRIVATE_KEY")`** — reads process env at runtime. Never hardcode. Never `--private-key 0x...` on the CLI either.
- **Python agents use `os.getenv("OPERATOR_PRIVATE_KEY")`** — same pattern. Never `dotenv.load_dotenv("~/.zshenv")`. Never shell out to echo env vars.
- **Check var presence without seeing value:** `[ -n "$VARNAME" ] && echo "set"` or `echo "${#VARNAME}"` (length only).
- **If a key ever surfaces in chat or output, STOP. Tell the user to rotate. Do not paginate the value back into context.**

Full playbook: `SECURITY.md`. Read it before any deploy or signing work.
