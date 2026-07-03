pragma circom 2.0.0;

include "bitify.circom";   // Num2Bits
include "poseidon.circom"; // circomlib Poseidon (bn128 / BN254 field only)

// Solvency circuit WITH a bound Merkle root (BN254 path).
//
// Proves, without revealing any balance:
//   1. every balance is a non-negative 64-bit amount (range check),
//   2. the balances sum to the public `total`, and
//   3. `root` is the Poseidon Merkle root of those exact same balances.
//
// Because the root is derived from the same private balances that produce the
// total, a customer's Poseidon inclusion proof against `root` is provably about
// the same set of liabilities the total was computed from. The contract's
// `verify_inclusion` uses the identical native Poseidon 2-to-1 hash.
//
// N must be a power of two.
template SolvencyBound(N) {
    signal input balances[N]; // private
    signal input total;       // public
    signal output root;       // public (Poseidon Merkle root of the balances)

    // range + sum
    component rc[N];
    signal sumAcc[N + 1];
    sumAcc[0] <== 0;
    for (var i = 0; i < N; i++) {
        rc[i] = Num2Bits(64);
        rc[i].in <== balances[i];
        sumAcc[i + 1] <== sumAcc[i] + balances[i];
    }
    total === sumAcc[N];

    // Poseidon Merkle root (leaves = balances). N=8: 4 -> 2 -> 1.
    component L1[N \ 2];
    for (var i = 0; i < N \ 2; i++) {
        L1[i] = Poseidon(2);
        L1[i].inputs[0] <== balances[2 * i];
        L1[i].inputs[1] <== balances[2 * i + 1];
    }
    component L2[N \ 4];
    for (var i = 0; i < N \ 4; i++) {
        L2[i] = Poseidon(2);
        L2[i].inputs[0] <== L1[2 * i].out;
        L2[i].inputs[1] <== L1[2 * i + 1].out;
    }
    component L3 = Poseidon(2);
    L3.inputs[0] <== L2[0].out;
    L3.inputs[1] <== L2[1].out;
    root <== L3.out;
}

component main { public [total] } = SolvencyBound(8);
