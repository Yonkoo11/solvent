#!/usr/bin/env bash
# Issuer tool: prove a set of (private) customer balances sums to a total and
# submit the attestation to the Solvent contract on Stellar testnet.
#
# Balances are used ONLY to generate the proof locally — they are never sent
# on-chain. Only the total, the reserve, and the proof leave this machine.
#
# Usage: scripts/attest.sh <reserve> <b1> <b2> ... <b8>
#   scripts/attest.sh 9000 1200 800 400 2500 100 900 1100 1000
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$ROOT"

CID="${SOLVENT_CONTRACT:-CAR32L3OU3W3CHQBWCN5IDTJZ5D4HL5EDIQEBLHTWOJE6OYVEBSALHBK}"
NET="${SOLVENT_NETWORK:-testnet}"
SRC="${SOLVENT_SOURCE:-issuer}"

RESERVE="$1"; shift
BAL=("$@")
[ "${#BAL[@]}" -eq 8 ] || { echo "need exactly 8 balances"; exit 1; }

# Sum + build circuit input (balances stay local).
TOTAL=0; for b in "${BAL[@]}"; do TOTAL=$((TOTAL + b)); done
printf '{ "balances": [%s], "total": "%s" }\n' \
  "$(printf '"%s",' "${BAL[@]}" | sed 's/,$//')" "$TOTAL" > circuits/input.json

echo "==> generating witness + proof locally (balances never leave this machine)"
node build/solvency_js/generate_witness.js build/solvency_js/solvency.wasm circuits/input.json build/witness.wtns
npx snarkjs groth16 prove build/solvency_final.zkey build/witness.wtns build/proof.json build/public.json
npx snarkjs groth16 verify build/verification_key.json build/public.json build/proof.json >/dev/null
echo "    proof verifies off-chain for total=$TOTAL"

echo "==> converting proof to on-chain byte format"
( cd scripts/converter && cargo run --quiet --bin convert >/dev/null )

PA=$(node -e "console.log(require('./build/invoke_args.json').proof_a)")
PB=$(node -e "console.log(require('./build/invoke_args.json').proof_b)")
PC=$(node -e "console.log(require('./build/invoke_args.json').proof_c)")
ISSUER=$(stellar keys address "$SRC")

echo "==> submitting attestation to $NET (total=$TOTAL reserve=$RESERVE)"
OUT=$(stellar contract invoke --id "$CID" --source "$SRC" --network "$NET" -- \
  attest --issuer "$ISSUER" --total "$TOTAL" --reserve "$RESERVE" \
  --proof_a "$PA" --proof_b "$PB" --proof_c "$PC" 2>&1)
echo "$OUT" | grep -iE 'expert/explorer|=\s*\{|^(true|false)$' | tail -3
RESULT=$(echo "$OUT" | tail -1)
if [ "$RESULT" = "true" ]; then echo "RESULT: ✅ SOLVENT (reserves cover proven liabilities of $TOTAL)"
elif [ "$RESULT" = "false" ]; then echo "RESULT: ❌ INSOLVENT (reserve $RESERVE < proven liabilities $TOTAL)"
else echo "RESULT: proof rejected or error"; fi
