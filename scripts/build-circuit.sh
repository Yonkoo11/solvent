#!/usr/bin/env bash
# Full Groth16 (BLS12-381) proving pipeline for the Solvency circuit.
# Everything lands in build/. Idempotent-ish: reuses the ptau if present.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
mkdir -p build
CLIB="node_modules/circomlib/circuits"
SNARKJS="npx snarkjs"

echo "==> [1/7] compile circuit for BLS12-381"
circom circuits/solvency.circom --r1cs --wasm -p bls12381 -l "$CLIB" -o build

echo "==> [2/7] powers of tau (bls12-381, power 12)"
if [ ! -f build/pot12_final.ptau ]; then
  $SNARKJS powersoftau new bls12-381 12 build/pot12_0000.ptau -v
  $SNARKJS powersoftau contribute build/pot12_0000.ptau build/pot12_0001.ptau \
    --name="solvent-1" -v -e="solvent entropy $(date +%s 2>/dev/null || echo 0)"
  $SNARKJS powersoftau prepare phase2 build/pot12_0001.ptau build/pot12_final.ptau -v
fi

echo "==> [3/7] groth16 trusted setup"
$SNARKJS groth16 setup build/solvency.r1cs build/pot12_final.ptau build/solvency_0000.zkey
$SNARKJS zkey contribute build/solvency_0000.zkey build/solvency_final.zkey \
  --name="solvent-key-1" -v -e="solvent zkey entropy"
$SNARKJS zkey export verificationkey build/solvency_final.zkey build/verification_key.json

echo "==> [4/7] witness from circuits/input.json"
node build/solvency_js/generate_witness.js build/solvency_js/solvency.wasm circuits/input.json build/witness.wtns

echo "==> [5/7] prove"
$SNARKJS groth16 prove build/solvency_final.zkey build/witness.wtns build/proof.json build/public.json

echo "==> [6/7] verify OFF-CHAIN (sanity)"
$SNARKJS groth16 verify build/verification_key.json build/public.json build/proof.json

echo "==> [7/7] public signals:"
cat build/public.json
echo
echo "DONE. Artifacts in build/: proof.json, public.json, verification_key.json, solvency_final.zkey"
